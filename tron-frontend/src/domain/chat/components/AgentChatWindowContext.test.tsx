// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import type { ChatAdapter, ChatSession } from "@tron/chat-core";
import type { ChatSession as UiSession } from "@personai/agent-ui-chat-core";
import { toUiSession } from "@tron/chat-agent-ui-adapter";
import { i18n } from "@/i18n";
import { OPEN_CHAT_CONTEXT_EVENT } from "@/shared/lib/shellEvents";
import { AgentChatWindow } from "./AgentChatWindow";

const captured = vi.hoisted(() => ({ change: null as ((session: UiSession | null) => void) | null }));
vi.mock("@personai/agent-ui-chat-react", () => ({
  ChatRuntimeWindow: (props: { footerAddon: ReactNode; onSessionChange: (session: UiSession | null) => void }) => {
    captured.change = props.onSessionChange;
    return <div>{props.footerAddon}</div>;
  },
}));
vi.mock("./ChatContextInspector", () => ({ ChatContextInspector: ({ session }: { session: ChatSession }) =>
  <div data-testid="inspected-session">{session.sessionCode}:{session.ownerDudeCode}</div> }));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn().mockResolvedValue({ snapshot: null }) }));
vi.mock("@/domain/chat/hooks/useChatInputSuggestions", () => ({ useChatInputSuggestions: () => ({}) }));
vi.mock("@/domain/chat/hooks/useChatFeedback", () => ({ useChatFeedback: () => ({ error: vi.fn(), info: vi.fn() }) }));
vi.mock("../hooks/useChatWorkRailLabels", () => ({ useChatWorkRailLabels: () => undefined, useChatHistoryLabels: () => undefined }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });

const session: ChatSession = { id: "dude-session", sessionCode: "dude-session", workspaceCode: null,
  ownerDudeCode: "main-dude", purpose: "Interactive", title: "Dude", status: "ACTIVE", createdAt: "", updatedAt: "" };
const adapter = { loadAguiEvents: async () => [] } as unknown as ChatAdapter;

it("retains Dude ownership after generic completion and opens its own context dialog", async () => {
  await i18n.changeLanguage("zh-CN");
  const opened = vi.fn();
  window.addEventListener(OPEN_CHAT_CONTEXT_EVENT, opened);
  try {
    const changed = vi.fn();
    render(<AgentChatWindow adapter={adapter} config={{ floating: true }} initialSession={session} onSessionChange={changed} />);
    const { ownerDudeCode: _owner, purpose: _purpose, ...generic } = session;
    for (const lastRunStatus of ["AWAITING_USER_INPUT", "COMPLETED"]) {
      await act(async () => captured.change?.(toUiSession({ ...generic, lastRunStatus })));
      expect(changed).toHaveBeenLastCalledWith(expect.objectContaining({ ownerDudeCode: "main-dude", purpose: "Interactive", lastRunStatus }));
    }
    fireEvent.click(screen.getByRole("button", { name: "上下文 0%" }));
    expect(await screen.findByTestId("inspected-session")).toHaveProperty("textContent", "dude-session:main-dude");
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(opened).not.toHaveBeenCalled();
  } finally { window.removeEventListener(OPEN_CHAT_CONTEXT_EVENT, opened); }
});

it("does not transfer Dude ownership to a different ordinary conversation", async () => {
  await i18n.changeLanguage("zh-CN");
  const opened = vi.fn();
  window.addEventListener(OPEN_CHAT_CONTEXT_EVENT, opened);
  try {
    render(<AgentChatWindow adapter={adapter} config={{ floating: false }} initialSession={session} />);
    const { ownerDudeCode: _owner, purpose: _purpose, ...generic } = session;
    await act(async () => captured.change?.(toUiSession({ ...generic, sessionCode: "ordinary", workspaceCode: "workspace" })));
    fireEvent.click(screen.getByRole("button", { name: "上下文 0%" }));
    expect(opened).toHaveBeenCalledOnce();
    expect((opened.mock.calls[0][0] as CustomEvent).detail.ownerDudeCode).toBeUndefined();
    expect(screen.queryByRole("dialog")).toBeNull();
  } finally { window.removeEventListener(OPEN_CHAT_CONTEXT_EVENT, opened); }
});

it("opens an embedded Dude session in the shared sidebar instead of a dialog", async () => {
  await i18n.changeLanguage("zh-CN");
  const opened = vi.fn();
  window.addEventListener(OPEN_CHAT_CONTEXT_EVENT, opened);
  try {
    render(<AgentChatWindow adapter={adapter} config={{ floating: false }} initialSession={session} />);
    const { ownerDudeCode: _owner, purpose: _purpose, ...generic } = session;
    await act(async () => captured.change?.(toUiSession({ ...generic, lastRunStatus: "COMPLETED" })));
    fireEvent.click(screen.getByRole("button", { name: "上下文 0%" }));
    expect(opened).toHaveBeenCalledOnce();
    expect((opened.mock.calls[0][0] as CustomEvent).detail).toMatchObject({ sessionCode: "dude-session", ownerDudeCode: "main-dude" });
    expect(screen.queryByRole("dialog")).toBeNull();
  } finally { window.removeEventListener(OPEN_CHAT_CONTEXT_EVENT, opened); }
});
