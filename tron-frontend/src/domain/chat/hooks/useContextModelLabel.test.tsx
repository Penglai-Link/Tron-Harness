// @vitest-environment jsdom
import { useState, type ReactNode } from "react";
import { cleanup, renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const fixture = vi.hoisted(() => ({ fetch: vi.fn(), user: "first-user" }));
vi.mock("@/domain/model/api", () => ({ fetchModelSelectOptions: fixture.fetch }));
vi.mock("@/store/authStore", () => ({ useAuthStore: (select: (state: unknown) => unknown) => select({ session: { user: { userCode: fixture.user } } }) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({
  t: (key: string) => key,
  i18n: { resolvedLanguage: "zh-CN" },
}) }));
import { useContextModelLabel } from "./useContextModelLabel";

function wrapper({ children }: { children: ReactNode }) {
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false } } }));
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
const option = (code: string, provider: string) => ({
  modelCode: code, displayName: "Model display name", modelName: "remote-model",
  accessPlatform: provider, sourceLabel: provider, vendor: provider, enabled: false,
});
beforeEach(() => { fixture.fetch.mockReset(); fixture.user = "first-user"; });
afterEach(cleanup);

it("resolves the saved report identity, including a model that is now disabled", async () => {
  fixture.fetch.mockResolvedValue([option("MODEL-saved", "Saved provider"), option("MODEL-current", "Current provider")]);
  const view = renderHook(() => useContextModelLabel("MODEL-saved", "workspace"), { wrapper });
  await waitFor(() => expect(view.result.current).toBe("Saved provider / Model display name"));
  expect(fixture.fetch).toHaveBeenCalledWith("workspace", expect.any(AbortSignal));
});

it("ignores a late lookup for a previously displayed model", async () => {
  let first!: (value: unknown) => void;
  let second!: (value: unknown) => void;
  fixture.fetch.mockImplementationOnce(() => new Promise(resolve => { first = resolve; }))
    .mockImplementationOnce(() => new Promise(resolve => { second = resolve; }));
  const view = renderHook(({ code }) => useContextModelLabel(code, "workspace"), { wrapper, initialProps: { code: "MODEL-first" } });
  view.rerender({ code: "MODEL-second" });
  expect(view.result.current).not.toContain("MODEL-");
  await act(async () => second([option("MODEL-second", "Second provider")]));
  await waitFor(() => expect(view.result.current).toBe("Second provider / Model display name"));
  await act(async () => first([option("MODEL-first", "First provider")]));
  expect(view.result.current).toBe("Second provider / Model display name");
});

it("does not reuse another account's cached model names", async () => {
  fixture.fetch.mockResolvedValueOnce([option("MODEL-same", "First account")])
    .mockResolvedValueOnce([option("MODEL-same", "Second account")]);
  const view = renderHook(() => useContextModelLabel("MODEL-same", "workspace"), { wrapper });
  await waitFor(() => expect(view.result.current).toBe("First account / Model display name"));
  fixture.user = "second-user";
  view.rerender();
  expect(view.result.current).not.toContain("First account");
  await waitFor(() => expect(view.result.current).toBe("Second account / Model display name"));
});

it("shows a localized unavailable label rather than an opaque code when lookup fails", async () => {
  fixture.fetch.mockRejectedValue(new Error("unavailable"));
  const view = renderHook(() => useContextModelLabel("MODEL-opaque", "workspace"), { wrapper });
  await waitFor(() => expect(view.result.current).toBe("chat.shortTermMemory.contextPanel.modelUnavailable"));
});

it("does not request model metadata without a report", () => {
  const view = renderHook(() => useContextModelLabel(null, "workspace"), { wrapper });
  expect(view.result.current).toBe("—");
  expect(fixture.fetch).not.toHaveBeenCalled();
});
