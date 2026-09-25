// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { CheckinButton } from "./CheckinButton";
import { checkinApi } from "../api/checkinApi";
import { fetchBootstrapProfileStatus } from "@/domain/profile/api/profileApi";

const mocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn(), update: vi.fn() }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => mocks }));
vi.mock("../api/checkinApi", () => ({ checkinApi: { status: vi.fn(), checkin: vi.fn() } }));
vi.mock("@/domain/profile/api/profileApi", () => ({ fetchBootstrapProfileStatus: vi.fn() }));
vi.mock("@/store/authStore", () => ({ useAuthStore: Object.assign(() => "user-1", { getState: () => ({ session: { user: { userCode: "user-1" } }, updateSessionUser: mocks.update }) }) }));
beforeEach(() => {
  vi.mocked(checkinApi.status).mockResolvedValue({ todayCheckedIn: false, todayRewardPoints: 10 });
  vi.mocked(fetchBootstrapProfileStatus).mockResolvedValue({ activeProfile: { personAiUserId: "user-1" }, account: { vipLevel: 1, remainingPoints: 110 } } as Awaited<ReturnType<typeof fetchBootstrapProfileStatus>>);
});
afterEach(() => { cleanup(); vi.resetAllMocks(); });

it("prevents duplicate clicks, shows the result and refreshes points", async () => {
  let finish!: (result: { points: number; continuousDays: number }) => void;
  vi.mocked(checkinApi.checkin).mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  render(<CheckinButton />);
  await waitFor(() => expect(checkinApi.status).toHaveBeenCalled());
  const button = screen.getByRole("button");
  fireEvent.click(button);
  fireEvent.click(button);
  expect(checkinApi.checkin).toHaveBeenCalledTimes(1);
  vi.mocked(checkinApi.status).mockResolvedValue({ todayCheckedIn: true, todayRewardPoints: 10 });
  await act(async () => { finish({ points: 10, continuousDays: 2 }); });
  await waitFor(() => expect(mocks.update).toHaveBeenCalledWith({ vipLevel: 1, remainingPoints: 110 }));
  expect(mocks.success).toHaveBeenCalledWith("userMenu.checkinSuccess");
  expect((button as HTMLButtonElement).disabled).toBe(true);
  expect(button.textContent).toBe("userMenu.checkedIn");
});

it("shows errors and recovers the already checked-in state from the server", async () => {
  render(<CheckinButton />);
  await waitFor(() => expect(checkinApi.status).toHaveBeenCalled());
  vi.mocked(checkinApi.checkin).mockRejectedValue(new Error("Already checked in"));
  vi.mocked(checkinApi.status).mockResolvedValue({ todayCheckedIn: true, todayRewardPoints: 10 });
  fireEvent.click(screen.getByRole("button"));
  await waitFor(() => expect(mocks.error).toHaveBeenCalledWith("Already checked in"));
  await waitFor(() => expect(screen.getByRole("button").textContent).toBe("userMenu.checkedIn"));
  expect(mocks.update).not.toHaveBeenCalled();
});
