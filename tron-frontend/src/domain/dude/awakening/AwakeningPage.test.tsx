// @vitest-environment jsdom
import { useEffect } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import AwakeningPage from "./AwakeningPage";
import type { ChatRuntimeRunState } from "@tron/chat-core";
const mocks = vi.hoisted(() => ({
  status: { dudeCode: "main-dude", name: "Neo", state: "AwaitingUser", started: true, canResume: true },
  invalidate: vi.fn().mockResolvedValue(undefined), defer: vi.fn().mockResolvedValue(undefined),
  run: { active: true, loading: false, sessionCode: "guide", runCode: "run", supportsSteer: false },
}));
vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@tanstack/react-query", () => ({ useQueryClient: () => client }));
const client = { invalidateQueries: mocks.invalidate };
vi.mock("../hooks/useDudes", () => ({ useDudes: () => ({ data: [] }) }));
vi.mock("@/domain/agent/components/AgentAvatarPicker", () => ({ AgentAvatar: () => <span>Avatar</span> }));
vi.mock("./BlackHole", () => ({ BlackHole: () => null }));
vi.mock("./awaitAwakeningRun", () => ({ awaitAwakeningRun: async () => {} }));
vi.mock("@/shared/motion/motionPreferences", () => ({ useMotionPreference: () => "system" }));
vi.mock("@tron/chat-internal-adapter", () => ({ createInternalChatAdapter: () => ({}) }));
vi.mock("../components/SoulFields", () => ({ SoulFields: () => null }));
vi.mock("../api/dudeApi", () => ({
  getAwakening: async () => ({ ...mocks.status }), beginAwakening: async () => ({ ...mocks.status }),
  resolveMainDudeSession: async () => ({ sessionCode: "guide", agentCode: "main-dude" }),
  deferAwakening: mocks.defer, getDudeSoul: vi.fn(), saveDudeSoul: vi.fn(), listDudeEvents: vi.fn(),
}));
vi.mock("@/domain/chat/components/AgentChatWindow", () => ({ AgentChatWindow: ({ onRunStateChange, onError }: {
  onRunStateChange: (s: ChatRuntimeRunState) => void; onError: (message: string) => void;
}) => {
  useEffect(() => onRunStateChange({ ...mocks.run }), [onRunStateChange]);
  return <><button onClick={() => onRunStateChange({ ...mocks.run, active: false, loading: false })}>Finish reply</button>
    <button onClick={() => onError("Model failed")}>Fail reply</button></>;
} }));
beforeEach(() => {
  vi.useFakeTimers(); vi.clearAllMocks();
  mocks.status.state = "AwaitingUser";
  mocks.run.active = true; mocks.run.loading = false;
  vi.stubGlobal("matchMedia", () => ({ matches: false }));
});
afterEach(() => { cleanup(); vi.useRealTimers(); vi.unstubAllGlobals(); });
async function open() {
  await act(async () => { render(<MemoryRouter initialEntries={["/awakening"]}><Routes>
    <Route path="/awakening" element={<AwakeningPage />} />
    <Route path="/workspace" element={<div>Home</div>} />
  </Routes></MemoryRouter>); });
}
it("waits for Ready and the final reply before automatically leaving, with a reading pause", async () => {
  await open();
  mocks.status.state = "Ready";
  await act(() => vi.advanceTimersByTimeAsync(6000));
  expect(screen.queryByText("Home")).toBeNull();
  fireEvent.click(screen.getByText("Finish reply"));
  await act(() => vi.advanceTimersByTimeAsync(2100));
  expect(screen.queryByText("Home")).toBeNull();
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(document.querySelector("main")?.dataset.phase).toBe("exit");
  await act(() => vi.advanceTimersByTimeAsync(1000));
  expect(screen.getByText("Home")).toBeTruthy();
  expect(mocks.defer).not.toHaveBeenCalled();
});
it("does not leave while restoring history or after a failed reply", async () => {
  mocks.run.active = false; mocks.run.loading = true;
  mocks.status.state = "Ready";
  await open();
  await act(() => vi.advanceTimersByTimeAsync(6000));
  expect(screen.queryByText("Home")).toBeNull();
  fireEvent.click(screen.getByText("Fail reply"));
  fireEvent.click(screen.getByText("Finish reply"));
  await act(() => vi.advanceTimersByTimeAsync(6000));
  expect(screen.getByRole("alert").textContent).toContain("Model failed");
  expect(screen.queryByText("Home")).toBeNull();
});
