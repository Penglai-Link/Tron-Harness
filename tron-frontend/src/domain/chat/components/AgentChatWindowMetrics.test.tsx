// @vitest-environment jsdom
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import type { ChatAdapter, ChatSession } from "@tron/chat-core";
import type { ChatRuntimeRunState } from "@personai/agent-ui-chat-core";
import { i18n } from "@/i18n";
import { requestApi } from "@/shared/api/httpClient";
import { AgentChatWindow } from "./AgentChatWindow";

const captured = vi.hoisted(() => ({ run: null as ((state: ChatRuntimeRunState) => void) | null }));
vi.mock("@personai/agent-ui-chat-react", () => ({
  ChatRuntimeWindow: (props: { footerAddon: ReactNode; onRunStateChange: (state: ChatRuntimeRunState) => void }) => {
    captured.run = props.onRunStateChange;
    return <div>{props.footerAddon}</div>;
  },
}));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn() }));
vi.mock("@/domain/chat/hooks/useChatInputSuggestions", () => ({ useChatInputSuggestions: () => ({}) }));
vi.mock("@/domain/chat/hooks/useChatFeedback", () => ({ useChatFeedback: () => ({ error: vi.fn(), info: vi.fn() }) }));
vi.mock("../hooks/useChatWorkRailLabels", () => ({ useChatWorkRailLabels: () => undefined, useChatHistoryLabels: () => undefined }));
beforeEach(() => { vi.useFakeTimers(); Object.defineProperty(document, "visibilityState", { configurable: true, value: "visible" }); });
afterEach(() => { cleanup(); vi.useRealTimers(); vi.clearAllMocks(); });

it("refreshes composer metrics during an active run and stops polling afterward", async () => {
  await i18n.changeLanguage("zh-CN");
  let input = 10000, cached = 2000, context = 20000, output = 100;
  const event = (eventName: string, occurredAt: string, metadata: Record<string, string>) => ({ id: eventName, type: eventName, eventName, runCode: "RUN", streamKey: "SESSION", occurredAt, outcome: "SUCCESS", metadata });
  const loadAguiEvents = vi.fn(async () => [
    event("agent.model.requested", "2026-10-27T12:00:00Z", { model_attempt_id: "A" }),
    event("agent.model.responded", "2026-10-27T12:00:02Z", { model_attempt_id: "A", usage_audit_json: JSON.stringify({ usageSource: "provider", usageConfidence: "actual", providerUsage: { inputTokens: input, cachedInputTokens: cached, outputTokens: output }, requestEstimate: { estimatedInputTokens: context, effectiveInputBudget: 100000 } }) }),
  ]);
  vi.mocked(requestApi).mockImplementation(async (endpoint) => endpoint.includes("get-context-assembly") ? { snapshot: { estimatedInputTokens: context, inputCapacity: 100000 } } as never : {} as never);
  const session: ChatSession = { id: 1, sessionCode: "SESSION", workspaceCode: "WORKSPACE", title: "Chat", status: "ACTIVE", createdAt: "", updatedAt: "" };
  const adapter = { loadAguiEvents } as unknown as ChatAdapter;
  await act(async () => { render(<AgentChatWindow adapter={adapter} config={{ floating: false }} initialSession={session} />); });
  expect(screen.getByText("— tok/s")).toBeTruthy();
  expect(screen.getByText("缓存 20%")).toBeTruthy();
  expect(screen.getByText("上下文 20%")).toBeTruthy();
  await act(async () => captured.run?.({ active: true, loading: false, sessionId: "SESSION", runId: "RUN", supportsSteer: false }));
  input = 20000; cached = 10000; context = 40000; output = 200;
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(screen.getByText("— tok/s")).toBeTruthy();
  expect(screen.getByText("缓存 50%")).toBeTruthy();
  expect(screen.getByText("上下文 40%")).toBeTruthy();
  await act(async () => captured.run?.({ active: false, loading: false, sessionId: "SESSION", runId: "RUN", supportsSteer: false }));
  const calls = loadAguiEvents.mock.calls.length;
  await act(async () => { await vi.advanceTimersByTimeAsync(6000); });
  expect(loadAguiEvents).toHaveBeenCalledTimes(calls);
});
