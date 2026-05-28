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
    [
      { type: "RUN_STARTED", threadId: session.sessionCode, runId, timestamp: startedAt },
      { type: "REASONING_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 1 },
      { type: "REASONING_MESSAGE_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 2 },
      { type: "REASONING_MESSAGE_CONTENT", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, delta: "Reviewing session context and available skills.", timestamp: startedAt + 3 },
      { type: "REASONING_MESSAGE_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 4 },
      { type: "REASONING_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 5 },
      { type: "TOOL_CALL_START", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool`, toolCallName: "workspace.search", timestamp: startedAt + 4 },
      { type: "ACTIVITY_DELTA", threadId: session.sessionCode, runId, delta: "Streaming workspace.search logs", timestamp: startedAt + 5 },
      { type: "TOOL_CALL_RESULT", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool`, content: "Matched workspace context", timestamp: startedAt + 5 },
      { type: "TOOL_CALL_END", threadId: session.sessionCode, runId, toolCallId: `${runId}-tool`, timestamp: startedAt + 6 },
      { type: "TEXT_MESSAGE_START", threadId: session.sessionCode, runId, messageId, role: "assistant", timestamp: startedAt + 7 },
    ].forEach((event) => handlers.onAguiEvent?.(event));
    if (assistant?.content) {
      assistant.content.match(/.{1,18}/g)?.forEach((delta, index) => {
        handlers.onAguiEvent?.({
          type: "TEXT_MESSAGE_CONTENT",
          threadId: session.sessionCode,
          runId,
          messageId,
          delta,
          timestamp: startedAt + 8 + index,
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
    if (type === "TEXT_MESSAGE_CONTENT" && payload.delta) {
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
      let boundary = buffer.indexOf("\n\n");
      while (boundary >= 0) {
        const frame = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        consumeFrame(frame);
        boundary = buffer.indexOf("\n\n");
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
