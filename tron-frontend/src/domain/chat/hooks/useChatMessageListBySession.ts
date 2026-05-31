import { getChatAguiEvents, getChatMessages, createMockChatSendResult } from "@/domain/chat/mock/data";
import { chatApiEndpoints } from "@/domain/chat/api";
import type {
  ChatAguiEvent,
  ChatAguiStreamEvent,
  ChatMessage,
  ChatSendResult,
  ChatSession,
  SendChatMessageInput,
} from "@/domain/chat/model";
import {
  buildApiUrl,
  getApiAccessToken,
  parseEndpoint,
  requestApi,
  unwrapApiEnvelope,
} from "@/shared/api/httpClient";
import {
  getServiceDataSource,
  useServiceQuery,
} from "@/shared/api/serviceConfig";

interface AguiEventSnapshot {
  events: ChatAguiEvent[];
}

export function useChatMessageListBySession(sessionCode: string) {
  return useServiceQuery<ChatMessage[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.listChatMessages,
    mockData: () => getChatMessages(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
  });
}

export function useChatAguiEvents(sessionCode: string) {
  return useServiceQuery<ChatAguiEvent[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.listAguiEvents,
    mockData: () => getChatAguiEvents(sessionCode),
    deps: [sessionCode],
    params: { streamKey: sessionCode },
    select: (payload) => {
      const snapshot = payload as AguiEventSnapshot | { data?: AguiEventSnapshot };
      if ("data" in snapshot && snapshot.data) return snapshot.data.events ?? [];
      return "events" in snapshot ? snapshot.events ?? [] : [];
    },
  });
}

export async function sendChatMessage(session: ChatSession, input: SendChatMessageInput): Promise<ChatSendResult> {
  if (getServiceDataSource("chat") === "mock") {
    return createMockChatSendResult(session, input);
  }

  return requestApi<ChatSendResult>(chatApiEndpoints.sendChatMessage, {
    body: input,
    pathParams: { sessionCode: session.sessionCode },
  });
}

interface StreamChatMessageHandlers {
  onAguiEvent?: (event: ChatAguiStreamEvent) => void;
  onDelta?: (delta: string) => void;
}

type ChatStreamPayload = ChatAguiStreamEvent & {
  type?: string;
};

export async function streamChatMessage(
  session: ChatSession,
  input: SendChatMessageInput,
  handlers: StreamChatMessageHandlers = {},
): Promise<ChatSendResult> {
  if (getServiceDataSource("chat") === "mock") {
    const result = createMockChatSendResult(session, input);
    const assistant = result.messages.find((message) => message.role === "ASSISTANT");
    const runId = result.run.runCode;
    const messageId = assistant?.messageCode ?? `${runId}-assistant`;
    const startedAt = Date.now();
    const staleRunId = `${runId}-stale`;
    const reasoningContent = [
      "Reviewing session context and available skills.",
      ...Array.from({ length: 18 }, (_, index) => `Reasoning stream segment ${index + 1}: validating latest summary visibility while the card content overflows.`),
    ];
    const reasoningContentEvents: ChatAguiStreamEvent[] = reasoningContent.map((delta, index) => ({
      type: index === 0 ? "REASONING_TEXT_CONTENT" : "REASONING_MESSAGE_CONTENT",
      threadId: session.sessionCode,
      runId,
      messageId: `${runId}-reasoning`,
      delta,
      timestamp: startedAt + 3 + index,
    }));
    const reasoningEndOffset = 3 + reasoningContentEvents.length;
    [
      { type: "RUN_STARTED", threadId: session.sessionCode, runId: staleRunId, timestamp: startedAt - 657_000 },
      { type: "REASONING_START", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 656_000 },
      { type: "REASONING_MESSAGE_START", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 655_000 },
      { type: "REASONING_MESSAGE_CONTENT", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, delta: "This stale run should not render.", timestamp: startedAt - 654_000 },
      { type: "REASONING_MESSAGE_END", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 653_000 },
      { type: "REASONING_END", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 652_000 },
      { type: "RUN_FINISHED", threadId: session.sessionCode, runId: staleRunId, timestamp: startedAt - 651_000 },
      { type: "RUN_STARTED", threadId: session.sessionCode, runId, timestamp: startedAt },
      { type: "REASONING_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 1 },
      { type: "REASONING_MESSAGE_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 2 },
      ...reasoningContentEvents,
      { type: "REASONING_MESSAGE_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + reasoningEndOffset },
      { type: "REASONING_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + reasoningEndOffset + 1 },
      { type: "TOOL_CALL_START", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool-1`, toolCallName: "shell.exec", toolCallArgs: { command: "rg AG-UI tron-frontend/src/domain/chat" }, timestamp: startedAt + reasoningEndOffset + 2 },
      { type: "TOOL_CALL_ARGS", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool-1`, toolCallName: "shell.exec", toolCallArgs: { command: "rg AG-UI tron-frontend/src/domain/chat" }, timestamp: startedAt + reasoningEndOffset + 3 },
      { type: "ACTIVITY_DELTA", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool-1`, delta: [{ op: "add", path: "/logs/-", value: "Scanning chat domain files" }], timestamp: startedAt + reasoningEndOffset + 4 },
      { type: "ACTIVITY_DELTA", threadId: session.sessionCode, runId, toolCallId: `${runId}-unmatched-tool`, delta: [{ op: "add", path: "/logs/-", value: "This log belongs elsewhere" }], timestamp: startedAt + reasoningEndOffset + 5 },
      { type: "TOOL_CALL_RESULT", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool-1`, toolCallName: "shell.exec", content: "Matched chat AG-UI presentation files", timestamp: startedAt + reasoningEndOffset + 6 },
      { type: "TOOL_CALL_END", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool-1`, timestamp: startedAt + reasoningEndOffset + 7 },
      { type: "TEXT_MESSAGE_START", threadId: session.sessionCode, runId, messageId, role: "assistant", timestamp: startedAt + reasoningEndOffset + 8 },
    ].forEach((event) => handlers.onAguiEvent?.(event));
    if (assistant?.content) {
      assistant.content.match(/.{1,18}/g)?.forEach((delta, index) => {
        handlers.onAguiEvent?.({
          type: "TEXT_MESSAGE_CONTENT",
          threadId: session.sessionCode,
          runId,
          messageId,
          delta,
          timestamp: startedAt + reasoningEndOffset + 9 + index,
        });
        handlers.onDelta?.(delta);
      });
    }
    handlers.onAguiEvent?.({
      type: "TEXT_MESSAGE_END",
      threadId: session.sessionCode,
      runId,
      messageId,
      timestamp: startedAt + 100,
    });
    handlers.onAguiEvent?.({
      type: "RUN_FINISHED",
      threadId: session.sessionCode,
      runId,
      result,
      timestamp: startedAt + 101,
    });
    return result;
  }

  const { path } = parseEndpoint(chatApiEndpoints.streamChatMessage);
  const headers = new Headers({ "Content-Type": "application/json" });
  const accessToken = getApiAccessToken();
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);

  const response = await fetch(buildApiUrl(path, { sessionCode: session.sessionCode }), {
    body: JSON.stringify(input),
    credentials: "include",
    headers,
    method: "POST",
  });

  if (!response.ok) {
    const payload = await response.json().catch(() => null);
    if (payload) return unwrapApiEnvelope<ChatSendResult>(payload);
    throw new Error(`POST ${response.url} failed with ${response.status}`);
  }
  if (!response.body) {
    throw new Error("Chat stream response did not include a body");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let result: ChatSendResult | null = null;

  function nextSseFrameBoundary(value: string): { index: number; separatorLength: number } | null {
    const lfBoundary = value.indexOf("\n\n");
    const crlfBoundary = value.indexOf("\r\n\r\n");
    if (lfBoundary < 0 && crlfBoundary < 0) return null;
    if (lfBoundary < 0) return { index: crlfBoundary, separatorLength: 4 };
    if (crlfBoundary < 0) return { index: lfBoundary, separatorLength: 2 };
    return lfBoundary < crlfBoundary
      ? { index: lfBoundary, separatorLength: 2 }
      : { index: crlfBoundary, separatorLength: 4 };
  }

  function consumeFrame(frame: string) {
    let eventName = "";
    const dataLines: string[] = [];
    frame.split(/\r?\n/).forEach((line) => {
      if (line.startsWith("event:")) eventName = line.slice(6).trim();
      if (line.startsWith("data:")) dataLines.push(line.slice(5).trimStart());
    });
    if (!dataLines.length) return;

    const payload = JSON.parse(dataLines.join("\n")) as ChatStreamPayload;
    const type = (payload.type ?? eventName).toUpperCase();
    const aguiEvent = { ...payload, type };
    handlers.onAguiEvent?.(aguiEvent);
    if (type === "TEXT_MESSAGE_CONTENT" && typeof payload.delta === "string") {
      handlers.onDelta?.(payload.delta);
      return;
    }
    if (type === "RUN_FINISHED" && payload.result) {
      result = payload.result;
      return;
    }
    if (type === "RUN_ERROR") {
      throw new Error(payload.message ?? payload.code ?? "Chat stream failed");
    }
  }

  while (!result) {
    const { done, value } = await reader.read();
    if (value) {
      buffer += decoder.decode(value, { stream: !done });
      let boundary = nextSseFrameBoundary(buffer);
      while (boundary) {
        const frame = buffer.slice(0, boundary.index);
        buffer = buffer.slice(boundary.index + boundary.separatorLength);
        consumeFrame(frame);
        boundary = nextSseFrameBoundary(buffer);
      }
    }
    if (done) break;
  }

  if (!result && buffer.trim()) consumeFrame(buffer);
  if (!result) throw new Error("Chat stream ended before completion");
  return result;
}

export function resolveChatAguiEventUrl(sessionCode: string) {
  return buildApiUrl("/api/v1/agents/agui/streams/{streamKey}/events", { streamKey: sessionCode });
}
