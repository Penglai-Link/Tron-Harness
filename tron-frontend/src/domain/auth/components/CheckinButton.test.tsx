// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { CheckinButton } from "./CheckinButton";
import { checkinApi } from "../api/checkinApi";
import { useAuthStore } from "@/store/authStore";

const mocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn(), refresh: vi.fn() }));
vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => mocks }));
vi.mock("../api/checkinApi", () => ({ checkinApi: { status: vi.fn(), checkin: vi.fn() } }));
beforeEach(() => {
  useAuthStore.setState({ status: "authenticated", refreshAccountSnapshot: mocks.refresh, session: {
    authMethod: "internal", authenticated: true, provider: null,
    user: { userCode: "user-1", username: "User", displayName: "User", email: "", avatarUrl: null, status: "active", vipLevel: 1, remainingPoints: 100, todayCheckedIn: false },
  } });
  mocks.refresh.mockResolvedValue(true);
});
afterEach(() => { cleanup(); vi.resetAllMocks(); });

it("uses the shared snapshot immediately across remounts without querying on open", () => {
  useAuthStore.getState().updateSessionUser({ todayCheckedIn: true });
  const view = render(<CheckinButton />);
  expect(screen.getByRole("button").textContent).toBe("userMenu.checkedIn");
  view.unmount();
  render(<CheckinButton />);
  expect(screen.getByRole("button").textContent).toBe("userMenu.checkedIn");
  expect(checkinApi.status).not.toHaveBeenCalled();
  expect(mocks.refresh).not.toHaveBeenCalled();
});

it("prevents duplicate clicks and preserves the balance until an actual snapshot arrives", async () => {
  let finish!: (result: { points: number; continuousDays: number }) => void;
  vi.mocked(checkinApi.checkin).mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  render(<CheckinButton />);
  const button = screen.getByRole("button");
  fireEvent.click(button);
  fireEvent.click(button);
  expect(checkinApi.checkin).toHaveBeenCalledTimes(1);
  await act(async () => { finish({ points: 10, continuousDays: 2 }); });
  expect(useAuthStore.getState().session?.user).toMatchObject({ remainingPoints: 100, todayCheckedIn: true });
  expect(mocks.refresh).toHaveBeenCalledTimes(1);
  expect(mocks.success).toHaveBeenCalledWith("userMenu.checkinSuccess");
  expect((button as HTMLButtonElement).disabled).toBe(true);
});

it("shows an error without clearing known account information", async () => {
  vi.mocked(checkinApi.checkin).mockRejectedValue(new Error("Unavailable"));
  render(<CheckinButton />);
  fireEvent.click(screen.getByRole("button"));
  await waitFor(() => expect(mocks.error).toHaveBeenCalledWith("Unavailable"));
  expect(useAuthStore.getState().session?.user).toMatchObject({ remainingPoints: 100, todayCheckedIn: false });
});

it("does not offer check-in before the first cached status is known", () => {
  useAuthStore.getState().updateSessionUser({ todayCheckedIn: null });
  render(<CheckinButton />);
  expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(true);
  expect(checkinApi.status).not.toHaveBeenCalled();
});

it("discards a successful response after switching accounts", async () => {
  let finish!: (result: { points: number; continuousDays: number }) => void;
  vi.mocked(checkinApi.checkin).mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  render(<CheckinButton />);
  fireEvent.click(screen.getByRole("button"));
  act(() => useAuthStore.getState().updateSessionUser({ userCode: "user-2" }));
  await act(async () => finish({ points: 10, continuousDays: 2 }));
  expect(useAuthStore.getState().session?.user.todayCheckedIn).toBe(false);
  expect(mocks.success).not.toHaveBeenCalled();
});
