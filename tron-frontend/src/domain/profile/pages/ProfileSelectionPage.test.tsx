// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import * as profileApi from "@/domain/profile/api";
import { useAuthStore } from "@/store/authStore";
import { ProfileSelectionPage } from "./ProfileSelectionPage";

vi.mock("@/domain/dude/awakening/BlackHole", () => ({ BlackHole: () => null }));
vi.mock("@/domain/dude/api/dudeApi", () => ({ getAwakening: async () => ({ started: true, state: "Ready" }) }));
vi.mock("@/shared/motion/motionPreferences", () => ({ useMotionPreference: () => "off" }));
vi.mock("react-i18next", () => {
  const t = (key: string) => key;
  return { useTranslation: () => ({ t }) };
});
vi.mock("@/i18n", () => ({ useLocale: () => ({ changeLocale: vi.fn(), currentLocale: "en-US", supportedLocales: [] }) }));
vi.mock("@/components/particles-bg/ParticlesBackground", () => ({ ParticlesBackground: () => null }));
vi.mock("@/shared/hooks/useManagedAvatar", () => ({ useManagedAvatar: () => ({ src: null }) }));
vi.mock("@/domain/profile/api", () => ({
  activateProfile: vi.fn(), createPendingProfile: vi.fn(), createPersonAiAuthorization: vi.fn(),
  fetchBootstrapProfileStatus: vi.fn(), fetchProfiles: vi.fn(), pollPersonAiAuthorization: vi.fn(), removeProfile: vi.fn(),
}));

const activeProfile: profileApi.PersonAiProfile = {
  profileId: "first-profile", personAiUserId: "user-1", personAiUsername: "First User", status: "ACTIVE",
  avatarUrl: null, createdAt: "2026-10-13T00:00:00Z", lastActivatedAt: null,
};
const pendingProfile = { ...activeProfile, status: "PENDING" };
const originalEnsureSessionFresh = useAuthStore.getState().ensureSessionFresh;
const session: Awaited<ReturnType<typeof profileApi.createPersonAiAuthorization>> = {
  providerCode: "PersonAi", sessionId: "authorization-1", loginUrl: "https://example.com/authorize",
  status: "PENDING", expiresAt: null, createdAt: 0, updatedAt: 0,
};
const bootstrap = (profile: profileApi.PersonAiProfile | null): profileApi.BootstrapProfileStatus => ({
  hasProfiles: !!profile, activeProfileId: profile?.profileId ?? null, activeProfile: profile, avatarUrl: null, account: null,
});

function Location() {
  return <output data-testid="location">{useLocation().pathname}{useLocation().search}</output>;
}

beforeEach(() => {
  useAuthStore.setState({ status: "unauthenticated", session: null });
  vi.spyOn(window, "open").mockReturnValue(null);
  vi.mocked(profileApi.createPendingProfile).mockResolvedValue(pendingProfile);
  vi.mocked(profileApi.createPersonAiAuthorization).mockResolvedValue(session);
});
afterEach(() => {
  cleanup();
  useAuthStore.setState({ ensureSessionFresh: originalEnsureSessionFresh, status: "unauthenticated", session: null });
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.resetAllMocks();
});

it.each(["first-login", "reauthorization", "failed-session"])("keeps %s in progress until session validation finishes", async (mode) => {
  const reauthorizing = mode === "reauthorization";
  let authorized = false;
  vi.mocked(profileApi.fetchProfiles).mockImplementation(async () => authorized ? [activeProfile] : reauthorizing ? [activeProfile] : []);
  vi.mocked(profileApi.fetchBootstrapProfileStatus).mockImplementation(async () => bootstrap(authorized || reauthorizing ? activeProfile : null));
  vi.mocked(profileApi.pollPersonAiAuthorization).mockImplementation(async () => {
    authorized = true;
    return { completed: true, session, connection: null, textVerified: null, embeddingVerified: null, profileId: activeProfile.profileId };
  });
  let finish!: (value: boolean) => void;
  const ensureSessionFresh = vi.fn(() => new Promise<boolean>((resolve) => { finish = resolve; }));
  useAuthStore.setState({ ensureSessionFresh });
  render(
    <MemoryRouter initialEntries={[reauthorizing ? "/profiles?mode=reauth" : "/profiles"]}>
      <Location />
      <Routes>
        <Route path="/profiles" element={<ProfileSelectionPage />} />
        <Route path="/workspace" element={<p>Workspace</p>} />
      </Routes>
    </MemoryRouter>,
  );
  if (!reauthorizing) {
    fireEvent.click(await screen.findByRole("checkbox"));
  }
  fireEvent.click(await screen.findByRole("button", { name: reauthorizing ? "personAiProfiles.reauthorize" : "personAiProfiles.authorizeNewAccount" }));
  await waitFor(() => expect(ensureSessionFresh).toHaveBeenCalledTimes(1));
  expect(screen.getByTestId("location").textContent).toBe(reauthorizing ? "/profiles?mode=reauth" : "/profiles");
  expect(screen.queryByText("personAiProfiles.reauthorizationRequired")).toBeNull();
  expect(screen.queryByRole("button", { name: "personAiProfiles.authorizeNewAccount" })).toBeNull();
  expect(screen.getByRole("button", { name: "personAiProfiles.signingIn" }).hasAttribute("disabled")).toBe(true);
  expect(screen.queryByRole("button", { name: "personAiProfiles.cancelAuthorization" })).toBeNull();
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval"] });
  await act(async () => {
    if (mode !== "failed-session") useAuthStore.setState({ status: "authenticated" });
    finish(mode !== "failed-session");
  });
  if (mode === "failed-session") {
    expect(screen.getByRole("alert").textContent).toBe("personAiProfiles.errors.activate");
    expect(screen.getByTestId("location").textContent).toBe("/profiles");
  } else {
    await act(() => vi.advanceTimersByTimeAsync(4300));
    expect(screen.getByText("Workspace")).toBeTruthy();
  }
});
