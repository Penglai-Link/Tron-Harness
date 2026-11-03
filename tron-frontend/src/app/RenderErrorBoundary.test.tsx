// @vitest-environment jsdom
import { lazy, type ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Link, MemoryRouter, Route, Routes } from "react-router";
import { createInstance } from "i18next";
import { I18nextProvider } from "react-i18next";
import { common as en } from "@/i18n/locales/en-US/common";
import { common as zh } from "@/i18n/locales/zh-CN/common";
import { RenderErrorBoundary } from "./RenderErrorBoundary";
import SuspendedRoute from "./route-page/SuspendedRoute";
import { lazyRoute, ROUTE_LOAD_TIMEOUT_MS } from "@/router/lazyRoute";

beforeEach(() => { vi.spyOn(console, "error").mockImplementation(() => {}); });
afterEach(() => { cleanup(); vi.useRealTimers(); vi.restoreAllMocks(); });

async function mount(children: ReactNode, locale = "zh-CN") {
  const i18n = createInstance();
  await i18n.init({ lng: locale, defaultNS: "common", resources: { "zh-CN": { common: zh }, "en-US": { common: en } } });
  return render(<I18nextProvider i18n={i18n}>{children}</I18nextProvider>);
}

function BrokenPage(): ReactNode { throw new Error("page crashed"); }

it("replaces an indefinitely pending module with recovery controls and preserves other tabs", async () => {
  vi.useFakeTimers();
  let finish!: (module: { default: () => ReactNode }) => void;
  const SlowPage = lazyRoute(() => new Promise<{ default: () => ReactNode }>((resolve) => { finish = resolve; }));
  await mount(<MemoryRouter>
    <input aria-label="Draft" defaultValue="Unsaved work" />
    <SuspendedRoute><SlowPage /></SuspendedRoute>
  </MemoryRouter>);
  expect(screen.getByRole("status").textContent).toContain(zh.table.loading);
  await act(() => vi.advanceTimersByTimeAsync(ROUTE_LOAD_TIMEOUT_MS));
  expect(screen.getByRole("alert").textContent).toContain(zh.renderError.title);
  expect(screen.getByRole("button", { name: zh.actions.refresh })).toBeTruthy();
  expect((screen.getByLabelText("Draft") as HTMLInputElement).value).toBe("Unsaved work");
  await act(async () => { finish({ default: () => <p>Late page</p> }); });
  expect(screen.queryByText("Late page")).toBeNull();
});

it("clears the module watchdog after a successful load", async () => {
  vi.useFakeTimers();
  const Page = lazyRoute(async () => ({ default: () => <p>Loaded page</p> }));
  await mount(<MemoryRouter><SuspendedRoute><Page /></SuspendedRoute></MemoryRouter>);
  await act(() => vi.advanceTimersByTimeAsync(ROUTE_LOAD_TIMEOUT_MS * 2));
  expect(screen.getByText("Loaded page")).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
  expect(console.error).not.toHaveBeenCalled();
});

it("contains a rejected lazy import without removing the shell or another tab's draft", async () => {
  const FailedPage = lazy(() => Promise.reject(new TypeError("Failed to fetch dynamically imported module")));
  await mount(<MemoryRouter>
    <nav>Navigation</nav>
    <input aria-label="Draft" defaultValue="Unsaved work" />
    <SuspendedRoute><FailedPage /></SuspendedRoute>
  </MemoryRouter>);
  expect((await screen.findByRole("alert")).textContent).toContain(zh.renderError.title);
  expect(screen.getByText("Navigation")).toBeTruthy();
  expect((screen.getByLabelText("Draft") as HTMLInputElement).value).toBe("Unsaved work");
  expect(screen.getByRole("button", { name: zh.actions.refresh })).toBeTruthy();
  expect(console.error).toHaveBeenCalled();
});

it("clears the failed route on navigation so another page can open without a reload", async () => {
  await mount(<MemoryRouter initialEntries={["/broken"]}>
    <Link to="/healthy">Open healthy page</Link>
    <SuspendedRoute><Routes>
      <Route path="/broken" element={<BrokenPage />} />
      <Route path="/healthy" element={<p>Healthy page</p>} />
    </Routes></SuspendedRoute>
  </MemoryRouter>);
  expect(screen.getByRole("alert")).toBeTruthy();
  fireEvent.click(screen.getByRole("link"));
  expect(await screen.findByText("Healthy page")).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
});

it.each(["zh-CN", "en-US"])("shows a translated recovery screen for shell failures in %s", async (locale) => {
  await mount(<RenderErrorBoundary fullScreen><BrokenPage /></RenderErrorBoundary>, locale);
  const messages = locale === "zh-CN" ? zh : en;
  expect(screen.getByRole("alert").textContent).toContain(messages.renderError.description);
  expect(screen.getByRole("button", { name: messages.actions.refresh })).toBeTruthy();
});
