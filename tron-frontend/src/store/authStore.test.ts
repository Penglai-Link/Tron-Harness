import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  bootstrap: vi.fn(), token: vi.fn(), setToken: vi.fn(), resetTabs: vi.fn(),
}));
vi.mock("@/domain/profile/api", () => ({
  fetchBootstrapProfileStatus: mocks.bootstrap,
  fetchActiveProfileToken: mocks.token,
}));
vi.mock("@/shared/api/httpClient", () => ({
  setApiAccessToken: mocks.setToken,
  setApiUnauthorizedHandler: vi.fn(),
  isUnauthorizedApiError: (error: unknown) => typeof error === "object"
    && error !== null && "status" in error && error.status === 401,
}));
vi.mock("@/store/tabStore", () => ({
  useTabStore: { getState: () => ({ resetAllTabs: mocks.resetTabs }) },
}));
import { useAuthStore } from "./authStore";
import { useTimeZoneStore } from "@/shared/lib/userTimeZone";

beforeEach(() => {
  vi.clearAllMocks();
  useAuthStore.setState({ session: null, status: "unauthenticated" });
  mocks.bootstrap.mockResolvedValue({
    activeProfile: { status: "ACTIVE", personAiUserId: "u", personAiUsername: "User" },
    avatarUrl: null, account: null,
  });
});

describe("401 recovery", () => {
  it("displays a nickname while preserving the account identifier", async () => {
    mocks.token.mockResolvedValue({ token: "fresh" });
    mocks.bootstrap.mockResolvedValue({ activeProfile: { status: "ACTIVE", personAiUserId: "u", personAiUsername: "account" }, avatarUrl: null, account: { nickname: "昵称", vipLevel: 1, remainingPoints: 12.5 } });
    expect(await useAuthStore.getState().ensureSessionFresh()).toBe(true);
    expect(useAuthStore.getState().session?.user).toMatchObject({ displayName: "昵称", username: "account", remainingPoints: 12.5 });
  });
  it("loads time zones from the database-backed bootstrap and resets on logout", async () => {
    mocks.token.mockResolvedValue({ token: "fresh" });
    mocks.bootstrap.mockResolvedValue({
      activeProfile: { status: "ACTIVE", personAiUserId: "u", personAiUsername: "User" },
      avatarUrl: null, account: null, timeZone: "America/New_York",
    });
    expect(await useAuthStore.getState().ensureSessionFresh()).toBe(true);
    expect(useTimeZoneStore.getState().timeZone).toBe("America/New_York");
    mocks.bootstrap.mockResolvedValue({
      activeProfile: { status: "ACTIVE", personAiUserId: "other", personAiUsername: "Other" },
      avatarUrl: null, account: null, timeZone: "Asia/Tokyo",
    });
    expect(await useAuthStore.getState().ensureSessionFresh()).toBe(true);
    expect(useTimeZoneStore.getState().timeZone).toBe("Asia/Tokyo");
    await useAuthStore.getState().logout();
    expect(useTimeZoneStore.getState().timeZone).toBe("Asia/Shanghai");
  });
  it("loads a refreshed token once for concurrent rejected requests", async () => {
    mocks.token.mockResolvedValue({ token: "fresh" });
    const first = useAuthStore.getState().recoverFromUnauthorized();
    const second = useAuthStore.getState().recoverFromUnauthorized();
    expect(first).toBe(second);
    expect(await first).toBe(true);
    expect(mocks.token).toHaveBeenCalledTimes(1);
    expect(mocks.setToken).toHaveBeenCalledWith("fresh");
    expect(useAuthStore.getState().status).toBe("authenticated");
  });

  it("does not replay requests when the active profile has invalid credentials", async () => {
    mocks.token.mockRejectedValue(Object.assign(new Error("unauthorized"), { status: 401 }));
    expect(await useAuthStore.getState().recoverFromUnauthorized()).toBe(false);
    expect(useAuthStore.getState().status).toBe("unauthenticated");
    expect(mocks.setToken).toHaveBeenCalledWith(null);
  });

  it("preserves an authenticated session when refresh is temporarily unavailable", async () => {
    mocks.token.mockResolvedValueOnce({ token: "previous" });
    expect(await useAuthStore.getState().ensureSessionFresh()).toBe(true);
    const session = useAuthStore.getState().session;
    mocks.setToken.mockClear();
    mocks.token.mockRejectedValueOnce(Object.assign(new Error("unavailable"), { status: 503 }));
    expect(await useAuthStore.getState().recoverFromUnauthorized()).toBe(false);
    expect(useAuthStore.getState().session).toBe(session);
    expect(useAuthStore.getState().status).toBe("authenticated");
    expect(mocks.setToken).not.toHaveBeenCalled();
    expect(mocks.resetTabs).not.toHaveBeenCalled();
  });
});
