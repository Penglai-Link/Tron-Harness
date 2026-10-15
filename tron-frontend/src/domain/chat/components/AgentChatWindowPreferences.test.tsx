// @vitest-environment jsdom
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ChatAdapter } from "@tron/chat-core";
import "@/i18n";
import { AgentChatWindow } from "./AgentChatWindow";

const captured = vi.hoisted(() => ({ config: null as { showReasoningCards?: boolean; runningEnterSubmitMode?: "steer" | "queue"; inputPlaceholder?: string; queuedInputPlaceholder?: string; steeringInputPlaceholder?: string } | null }));
vi.mock("@eliteforge/agent-ui-chat-react", () => ({
  ChatRuntimeWindow: (props: { config: { showReasoningCards?: boolean; runningEnterSubmitMode?: "steer" | "queue"; inputPlaceholder?: string; queuedInputPlaceholder?: string; steeringInputPlaceholder?: string } }) => {
    captured.config = props.config;
    return <div data-testid="chat-runtime" />;
  },
}));
vi.mock("@tron/chat-agent-ui-adapter", () => ({ createAgentUiAdapter: () => ({}) }));
vi.mock("@/domain/chat/hooks/useChatInputSuggestions", () => ({ useChatInputSuggestions: () => ({}) }));
vi.mock("@/domain/chat/hooks/useChatFeedback", () => ({ useChatFeedback: () => ({ error: vi.fn(), info: vi.fn() }) }));
vi.mock("../hooks/useChatWorkRailLabels", () => ({ useChatWorkRailLabels: () => undefined, useChatHistoryLabels: () => undefined }));

afterEach(() => {
  cleanup();
  window.localStorage.removeItem("tron.aguiWorkReplayPreferences");
  captured.config = null;
});

it("updates an open chat when work preferences change and removes composer hints", () => {
  render(<AgentChatWindow adapter={{} as ChatAdapter} config={{ floating: false }} />);
  expect(captured.config?.showReasoningCards).toBe(false);
  expect(captured.config?.runningEnterSubmitMode).toBe("steer");
  expect(captured.config?.inputPlaceholder).toBe("");
  expect(captured.config?.queuedInputPlaceholder).toBe("");
  expect(captured.config?.steeringInputPlaceholder).toBe("");

  act(() => {
    window.localStorage.setItem("tron.aguiWorkReplayPreferences", JSON.stringify({ recentTurnLimit: 5, showReasoningCards: true, runningEnterSubmitMode: "queue" }));
    window.dispatchEvent(new CustomEvent("tron.aguiWorkReplayPreferences.changed"));
  });
  expect(captured.config?.showReasoningCards).toBe(true);
  expect(captured.config?.runningEnterSubmitMode).toBe("queue");
});
