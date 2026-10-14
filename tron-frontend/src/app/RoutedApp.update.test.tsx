// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { Link, MemoryRouter } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import RoutedApp from "./RoutedApp";

const mocks = vi.hoisted(() => ({
  check: vi.fn().mockResolvedValue({ type: "upToDate" }),
  auth: { status: "authenticated", session: { authenticated: true } },
}));
vi.mock("@/domain/auth", () => ({ useBootstrapAuthSession: () => {}, useAuthSession: () => mocks.auth }));
vi.mock("@tanstack/react-query", () => ({ useQueryClient: () => ({ clear: vi.fn() }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: vi.fn() }) }));
vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/domain/update/useUpdater", () => ({ useUpdater: () => ({
  isDesktop: true, check: mocks.check, status: { type: "idle" }, apply: vi.fn(), startDownload: vi.fn(),
}) }));
vi.mock("@/shared/layout/content/TabPanelRegistry", () => ({ TabPanelRegistryProvider: ({ children }: { children: ReactNode }) => children }));
vi.mock("@/shared/layout/content/FilePanelHostProvider", () => ({ FilePanelHostProvider: ({ children }: { children: ReactNode }) => children }));
vi.mock("@/domain/dude/awakening/HomeArrivalVeil", () => ({ HomeArrivalVeil: ({ children }: { children: ReactNode }) => children }));
vi.mock("@/domain/dude/awakening/AwakeningPage", () => ({ default: () => <div>Awakening</div> }));
vi.mock("@/domain/profile/pages/ProfileSelectionPage", () => ({ ProfileSelectionPage: () => <div>Login</div> }));
vi.mock("./route-page/EntryRoutePage", () => ({ default: () => <div>Entry</div> }));
vi.mock("./route-page/AppRoutePage", () => ({ default: () => <div>Workspace</div> }));
vi.mock("./route-page/RouteLoadingFallback", () => ({ default: () => <div>Checking session</div> }));

beforeEach(() => {
  vi.useFakeTimers();
  mocks.check.mockClear();
  mocks.auth.status = "authenticated";
  vi.stubGlobal("EventSource", class { addEventListener() {} removeEventListener() {} close() {} });
});
afterEach(() => { cleanup(); vi.useRealTimers(); vi.unstubAllGlobals(); });
async function open(path: string) {
  await act(async () => {
    render(<MemoryRouter initialEntries={[path]}>
      <Link to="/workspace">Enter</Link><Link to="/awakening">Return to awakening</Link>
      <RoutedApp />
    </MemoryRouter>);
  });
}
it.each(["/", "/profiles", "/awakening"])("does not check updates on %s even with a valid session", async (path) => {
  await open(path);
  await act(() => vi.advanceTimersByTimeAsync(10000));
  expect(mocks.check).not.toHaveBeenCalled();
});
it("only checks after entering the system, and cancels the delay when leaving", async () => {
  await open("/awakening");
  fireEvent.click(screen.getByText("Enter"));
  await act(() => vi.advanceTimersByTimeAsync(2000));
  expect(mocks.check).not.toHaveBeenCalled();
  fireEvent.click(screen.getByText("Return to awakening"));
  await act(() => vi.advanceTimersByTimeAsync(5000));
  expect(mocks.check).not.toHaveBeenCalled();
  fireEvent.click(screen.getByText("Enter"));
  await act(() => vi.advanceTimersByTimeAsync(2500));
  expect(mocks.check).toHaveBeenCalledOnce();
});
it("does not check while the session is still being verified", async () => {
  mocks.auth.status = "checking";
  await open("/workspace");
  await act(() => vi.advanceTimersByTimeAsync(10000));
  expect(mocks.check).not.toHaveBeenCalled();
});
