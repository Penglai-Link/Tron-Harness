// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import type { ChatSession } from "@tron/chat-core";
import { i18n } from "@/i18n";
import { useTabStore } from "@/store/tabStore";
import { TabPanelRegistryProvider } from "@/shared/layout/content/TabPanelRegistry";
import RightPanelContainer from "@/shared/layout/content/RightPanelContainer";
import { DudesPage } from "./DudesPage";

const api = vi.hoisted(() => ({ resolve: vi.fn(), sessions: vi.fn() }));
vi.mock("../api/dudeApi", () => ({
  dudeQueryKeys: { all: ["dudes"], sessions: (code: string) => ["dude-sessions", code] },
  listDudes: async () => [
    { dudeCode: "main-dude", name: "Main Dude", isMain: true, enabled: true },
    { dudeCode: "memory-dude", name: "Memory Dude", enabled: true, kind: "Memory" },
  ],
  resolveDudeSession: (code: string) => api.resolve(code),
  listDudeSessions: (code: string) => api.sessions(code),
  listDudeEvents: async () => [],
}));
vi.mock("@/domain/chat/components/AgentChatWindow", () => ({ AgentChatWindow: () => <div>Chat</div> }));
vi.mock("@/domain/chat/components/ChatContextInspector", () => ({ ChatContextInspector: ({ session }: { session: ChatSession }) =>
  <div data-testid="inspected-session">{session.ownerDudeCode}:{session.sessionCode}</div> }));
vi.mock("@/domain/chat", () => ({ useChatHistoryLabels: () => undefined, useChatWorkRailLabels: () => undefined }));
vi.mock("@/domain/auth", () => ({ useAuthSession: () => ({ session: null }) }));
vi.mock("@tron/chat-internal-adapter", () => ({ createInternalChatAdapter: () => ({}) }));
// Exercise session/inspector ownership independently of floating menu layout.
// The real switcher has its own component tests and keyboard browser checks.
vi.mock("../components/DudeSwitcher", () => ({ DudeSwitcher: ({ dudes, onValueChange }: {
  dudes: { dudeCode: string; name: string }[]; onValueChange: (code: string) => void;
}) => <>{dudes.map(dude => <button key={dude.dudeCode} onClick={() => onValueChange(dude.dudeCode)}>{dude.name}</button>)}</> }));
vi.mock("../components/DudeThreads", () => ({ DudeThreads: () => null }));
vi.mock("../components/DudeTasks", () => ({ DudeTasks: () => null }));
vi.mock("@/domain/agent/components/AgentAvatarPicker", () => ({ AgentAvatar: () => null }));

const session = (ownerDudeCode: string, sessionCode: string, purpose = "Interactive") => ({
  id: sessionCode, sessionCode, ownerDudeCode, purpose, workspaceCode: null,
  title: sessionCode, status: "ACTIVE", createdAt: "2026-10-26T00:00:00Z", updatedAt: "2026-10-26T00:00:00Z",
});

afterEach(() => { cleanup(); useTabStore.getState().clearAllTabs(); vi.clearAllMocks(); });

it("registers short-term memory for the selected Dude and history session without leaking the former owner", async () => {
  await i18n.changeLanguage("zh-CN");
  let resolveMemory!: (value: ReturnType<typeof session>) => void;
  api.resolve.mockImplementation((code: string) => code === "main-dude"
    ? Promise.resolve(session(code, "main-session"))
    : new Promise(resolve => { resolveMemory = resolve; }));
  api.sessions.mockImplementation(async (code: string) => [session(code, `${code}-history`, "Task")]);
  const query = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={query}><MemoryRouter initialEntries={["/dudes"]}>
    <TabPanelRegistryProvider><DudesPage /><RightPanelContainer /></TabPanelRegistryProvider>
  </MemoryRouter></QueryClientProvider>);
  expect(await screen.findByRole("tab", { name: "短期记忆" })).toBeTruthy();
  expect(await screen.findByTestId("inspected-session")).toHaveProperty("textContent", "main-dude:main-session");
  expect(screen.queryByTestId("tron-right-sidebar-empty")).toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "Memory Dude" }));
  await waitFor(() => expect(api.resolve).toHaveBeenCalledWith("memory-dude"));
  expect(screen.queryByTestId("inspected-session")).toBeNull();
  resolveMemory(session("memory-dude", "memory-session"));
  expect(await screen.findByTestId("inspected-session")).toHaveProperty("textContent", "memory-dude:memory-session");

  fireEvent.click(screen.getByRole("tab", { name: "历史会话" }));
  fireEvent.click(await screen.findByRole("button", { name: "打开" }));
  expect(await screen.findByTestId("inspected-session")).toHaveProperty("textContent", "memory-dude:memory-dude-history");
  fireEvent.click(screen.getByRole("tab", { name: "任务" }));
  await waitFor(() => expect(screen.queryByTestId("inspected-session")).toBeNull());
  query.clear();
});
