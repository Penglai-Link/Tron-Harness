// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { MainAgentChatWindow } from "./MainAgentChatWindow";
const { resolve, adapter, lastProps } = vi.hoisted(() => ({ resolve: vi.fn(), adapter: vi.fn((_options: {getHeaders: () => Headers; workspaceCode: string | null}) => ({})), lastProps: { current: null as unknown } }));
vi.mock("@/domain/dude/api/dudeApi", () => ({ resolveMainDudeSession: resolve }));
vi.mock("@tron/chat-internal-adapter", () => ({ createInternalChatAdapter: adapter }));
vi.mock("@/domain/chat/components/AgentChatWindow", () => ({ AgentChatWindow: (props: { initialSession: ChatSession }) => { lastProps.current = props; return <div data-testid="main-dude-session">{props.initialSession.sessionCode}</div>; } }));
vi.mock("@/domain/chat", () => ({ useChatWorkRailLabels: () => undefined, useChatHistoryLabels: () => undefined }));
vi.mock("@/shared", () => ({ useTheme: () => ({ agentReplyEffect: "none", agentReplySpeed: "normal" }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: vi.fn() }) }));
vi.mock("@/domain/chat/hooks/useChatFeedback", () => ({ useChatFeedback: () => ({ error: vi.fn(), info: vi.fn() }) }));
vi.mock("@/shared/desktop/DesktopBridge", () => ({ DesktopBridge: class { isDesktop() { return false; } } }));
vi.mock("@/components/markdown-link", () => ({ MarkdownLink: () => null }));
vi.mock("react-i18next", async importOriginal => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => key }) }));
const session: ChatSession = { id: "9007199254740993", sessionCode: "platform-session", workspaceCode: null, agentCode: "main-dude", ownerDudeCode: "main-dude", purpose: "Interactive", title: "Main Dude", status: "ACTIVE", createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" };
const windowState = { open: true, collapsed: false, x: 0, y: 0, width: 400, height: 600 };
afterEach(() => { cleanup(); vi.clearAllMocks(); localStorage.clear(); });
it("opens the server-resolved platform session and preserves it while the selected workspace changes", async () => {
  resolve.mockResolvedValue(session);
  const view = render(<MainAgentChatWindow workspaceCode="alpha" windowState={windowState} onWindowStateChange={vi.fn()} />);
  expect(await screen.findByTestId("main-dude-session")).toHaveProperty("textContent", "platform-session");
  view.rerender(<MainAgentChatWindow workspaceCode="beta" windowState={windowState} onWindowStateChange={vi.fn()} />);
  expect(resolve).toHaveBeenCalledTimes(1);
  expect(adapter).toHaveBeenCalledWith(expect.objectContaining({ workspaceCode: null }));
  const headers = await adapter.mock.calls.at(-1)![0].getHeaders();
  expect(headers.get("X-Dude-Code")).toBe("main-dude");
  expect(headers.has("X-Workspace-Code")).toBe(false);
  const props = lastProps.current as { config: {showNewSession:boolean}; createSendInput: (session: ChatSession, content: string, files: []) => SendChatMessageInput };
  expect(props.config.showNewSession).toBe(false);
  expect(props.createSendInput(session, "hello", [])).toMatchObject({ workspaceCode: null, agentCode: "main-dude", content: "hello" });
  expect(localStorage.getItem("tron.main-agent.session:alpha")).toBeNull();
});
it("resolves again after reopening so a changed main Dude takes effect without replacing the session locally", async () => {
  resolve.mockResolvedValueOnce(session).mockResolvedValueOnce({ ...session, sessionCode: "replacement-session", agentCode: "another-dude" });
  const first = render(<MainAgentChatWindow workspaceCode="alpha" windowState={windowState} onWindowStateChange={vi.fn()} />);
  await screen.findByText("platform-session"); first.unmount();
  render(<MainAgentChatWindow workspaceCode="beta" windowState={windowState} onWindowStateChange={vi.fn()} />);
  await screen.findByText("replacement-session");
  expect(resolve).toHaveBeenCalledTimes(2);
});
it("keeps Dude authorization after generic chat completions and AskUser resumes", async () => {
  resolve.mockResolvedValue(session);
  render(<MainAgentChatWindow windowState={windowState} onWindowStateChange={vi.fn()} />);
  await screen.findByText("platform-session");
  const { ownerDudeCode: _owner, purpose: _purpose, ...completion } = session;
  for (const lastRunStatus of ["AWAITING_USER_INPUT", "COMPLETED"]) {
    const props = lastProps.current as { onSessionChange: (value: ChatSession) => void };
    await act(async () => props.onSessionChange({ ...completion, lastRunStatus }));
    const headers = adapter.mock.calls.at(-1)![0].getHeaders();
    expect(headers.get("X-Dude-Code")).toBe("main-dude");
    expect(headers.has("X-Workspace-Code")).toBe(false);
    expect((lastProps.current as {initialSession: ChatSession}).initialSession).toMatchObject({
      ownerDudeCode: "main-dude", purpose: "Interactive", lastRunStatus,
    });
  }
});
it("keeps resolution failures visible and never opens a fallback workspace session", async () => {
  resolve.mockRejectedValue(new Error("Platform session unavailable"));
  render(<MainAgentChatWindow workspaceCode="alpha" windowState={windowState} onWindowStateChange={vi.fn()} />);
  await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Platform session unavailable"));
  expect(screen.queryByTestId("main-dude-session")).toBeNull();
});
