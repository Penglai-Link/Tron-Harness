// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import * as profileApi from "@/domain/profile/api";
import { getAwakening, type AwakeningStatus } from "@/domain/dude/api/dudeApi";
import { useAuthStore } from "@/store/authStore";
import { ProfileSelectionPage } from "./ProfileSelectionPage";

vi.mock("@/domain/dude/awakening/BlackHole", () => ({ BlackHole: () => <div data-testid="black-hole" /> }));
vi.mock("@/domain/dude/api/dudeApi", () => ({ getAwakening: vi.fn() }));
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
  localStorage.clear();
  vi.mocked(getAwakening).mockResolvedValue({
    dudeCode: "main-dude", name: "Neo", state: "Ready", started: true,
    deferred: false, sessionCode: "guide", canResume: true,
  });
  useAuthStore.setState({ status: "unauthenticated", session: null });
  vi.spyOn(window, "open").mockReturnValue(null);
  vi.mocked(profileApi.createPendingProfile).mockResolvedValue(pendingProfile);
  vi.mocked(profileApi.createPersonAiAuthorization).mockResolvedValue(session);
});

it.each<{
  label: string;
  state: AwakeningStatus["state"];
  started: boolean;
  deferred: boolean;
  canResume: boolean;
  destination: string;
}>([
  { label: "never started", state: "Uninitialized", started: false, deferred: false, canResume: false, destination: "/awakening?arrival=inside" },
  { label: "closed before answering", state: "AwaitingUser", started: true, deferred: false, canResume: true, destination: "/awakening?arrival=inside" },
  { label: "closed before the first question", state: "Uninitialized", started: true, deferred: false, canResume: true, destination: "/awakening?arrival=inside" },
  { label: "unfinished without a resumable private session", state: "AwaitingUser", started: true, deferred: false, canResume: false, destination: "/awakening?arrival=inside" },
  { label: "explicitly skipped", state: "AwaitingUser", started: true, deferred: true, canResume: true, destination: "/workspace" },
  { label: "finished", state: "Ready", started: true, deferred: false, canResume: true, destination: "/workspace" },
])("routes returning users correctly when initialization was $label", async ({ label: _, destination, ...status }) => {
  vi.useFakeTimers();
  vi.mocked(getAwakening).mockResolvedValue({ dudeCode: "main-dude", name: "Neo", sessionCode: "guide", ...status });
  vi.mocked(profileApi.fetchProfiles).mockResolvedValue([activeProfile]);
  vi.mocked(profileApi.fetchBootstrapProfileStatus).mockResolvedValue(bootstrap(activeProfile));
  useAuthStore.setState({ status: "authenticated" });
  await act(async () => {
    render(<MemoryRouter initialEntries={["/profiles"]}>
      <Location />
      <Routes>
        <Route path="/profiles" element={<ProfileSelectionPage />} />
        <Route path="/workspace" element={<p>Workspace</p>} />
        <Route path="/awakening" element={<p>Resume initialization</p>} />
      </Routes>
    </MemoryRouter>);
  });
  if (status.state === "Ready") {
    // No approach, dive, or arrival delay for an initialized user's default.
    expect(screen.getByTestId("location").textContent).toBe("/workspace");
    expect(screen.queryByTestId("black-hole")).toBeNull();
  } else {
    expect(screen.getByTestId("location").textContent).toBe("/profiles");
    expect(screen.getByTestId("black-hole")).toBeTruthy();
  }
  await act(() => vi.advanceTimersByTimeAsync(4300));
  expect(getAwakening).toHaveBeenCalledOnce();
  expect(screen.getByTestId("location").textContent).toBe(destination);
});

it.each([null, "false"])("keeps the black hole before first authorization with entrance preference %s", async (preference) => {
  if (preference !== null) localStorage.setItem("tron.app-entrance-animation", preference);
  vi.useFakeTimers();
  vi.mocked(profileApi.fetchProfiles).mockResolvedValue([]);
  vi.mocked(profileApi.fetchBootstrapProfileStatus).mockResolvedValue(bootstrap(null));
  await act(async () => {
    render(<MemoryRouter><ProfileSelectionPage /></MemoryRouter>);
  });
  expect(screen.getByTestId("black-hole")).toBeTruthy();
  expect(document.querySelector('[data-camera-phase="approach"]')).toBeTruthy();
  expect(document.querySelector('[data-tron-entry-ready]')).toBeNull();
  expect(getAwakening).not.toHaveBeenCalled();
  await act(() => vi.advanceTimersByTimeAsync(2400));
  expect(document.querySelector('[data-camera-phase="login"]')).toBeTruthy();
  expect(screen.getByTestId("black-hole")).toBeTruthy();
});

it("plays the entrance on returning when explicitly enabled", async () => {
  localStorage.setItem("tron.app-entrance-animation", "true");
  vi.useFakeTimers();
  vi.mocked(profileApi.fetchProfiles).mockResolvedValue([activeProfile]);
  vi.mocked(profileApi.fetchBootstrapProfileStatus).mockResolvedValue(bootstrap(activeProfile));
  useAuthStore.setState({ status: "authenticated" });
  await act(async () => {
    render(<MemoryRouter initialEntries={["/profiles"]}><Location /><Routes>
      <Route path="/profiles" element={<ProfileSelectionPage />} />
      <Route path="/workspace" element={<p>Workspace</p>} />
    </Routes></MemoryRouter>);
  });
  expect(screen.getByTestId("location").textContent).toBe("/profiles");
  expect(screen.getByTestId("black-hole")).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(2400));
  expect(document.querySelector('[data-camera-phase="dive"]')).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(1800));
  expect(screen.getByText("Workspace")).toBeTruthy();
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
