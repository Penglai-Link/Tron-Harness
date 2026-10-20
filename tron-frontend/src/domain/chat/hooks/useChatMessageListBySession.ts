import { tracedFetch } from "@tron/chat-core";
import { getChatAguiEvents, getChatMessages, createMockChatSendResult } from "@/domain/chat/mock/data";
import { chatApiEndpoints } from "@/domain/chat/api";
import type {
  ChatAguiEvent,
  ChatAguiStreamEvent,
  ChatMessage,
  ChatSendResult,
  ChatSession,
  ChatSteerResult,
  ChatUserInputAnswerInput,
  ChatUserInputRequest,
  GetActiveChatRunResult,
  SendChatMessageInput,
  SteerChatMessageInput,
  StopChatRunInput,
  StopChatRunResult,
} from "@/domain/chat/model";
import { normalizeChatUserInputRequest } from "@tron/chat-core";
import { applyCurrentLocaleHeader } from "@/i18n";
import {
  ApiError,
  buildApiUrl,
  getApiAccessToken,
  parseEndpoint,
  requestApi,
} from "@/shared/api/httpClient";
import {
  getServiceDataSource,
  useServiceQuery,
  useServiceQueryState,
} from "@/shared/api/serviceConfig";

interface AguiEventSnapshot {
  events: ChatAguiEvent[];
}

interface AguiProtocolEvent {
  sseEvent: string;
  data: ChatAguiStreamEvent;
}

interface ChatMessageListQueryOptions {
  suppressErrorToast?: boolean;
  dudeCode?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function useChatMessageListBySession(sessionCode: string, refreshKey = 0, options: ChatMessageListQueryOptions = {}) {
  return useServiceQuery<ChatMessage[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.listChatMessages,
    headers: options.dudeCode ? { "X-Dude-Code": options.dudeCode } : undefined,
    mockData: () => getChatMessages(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
    suppressErrorToast: options.suppressErrorToast,
  });
}

export function useChatAguiEvents(sessionCode: string, refreshKey = 0, dudeCode?: string) {
  return useServiceQuery<ChatAguiEvent[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.listAguiEvents,
    headers: dudeCode ? { "X-Dude-Code": dudeCode } : undefined,
    mockData: () => getChatAguiEvents(sessionCode),
    deps: [sessionCode],
    params: { streamKey: sessionCode },
    refreshKey,
    select: (payload) => {
      const snapshot = payload as AguiEventSnapshot | { data?: AguiEventSnapshot };
      if ("data" in snapshot && snapshot.data) return snapshot.data.events ?? [];
      return "events" in snapshot ? snapshot.events ?? [] : [];
    },
  });
}

export function useChatAguiProtocolEvents(sessionCode: string, refreshKey = 0) {
  return useServiceQuery<ChatAguiStreamEvent[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.listAguiProtocolEvents,
    mockData: () => [],
    deps: [sessionCode],
    params: { streamKey: sessionCode },
    refreshKey,
    select: (payload) => {
      const records = Array.isArray(payload)
        ? payload
        : ("data" in (payload as { data?: unknown }) && Array.isArray((payload as { data?: unknown }).data)
            ? (payload as { data: unknown[] }).data
            : []);
      return (records as AguiProtocolEvent[]).map((event) => ({
        ...event.data,
        type: (event.data?.type ?? event.sseEvent).toUpperCase(),
      }));
    },
  });
}

export function useChatActiveRunState(sessionCode: string, refreshKey = 0) {
  return useServiceQueryState<GetActiveChatRunResult>({
    apiInitialData: () => ({ activeRun: null, sessionCode }),
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getActiveChatRun,
    mockData: () => ({ activeRun: null, sessionCode }),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
    suppressErrorToast: true,
  });
}

export async function getChatActiveRun(
  sessionCode: string,
  signal?: AbortSignal,
): Promise<GetActiveChatRunResult> {
  if (getServiceDataSource("chat") === "mock") {
    return { activeRun: null, sessionCode };
  }
  return requestApi<GetActiveChatRunResult>(chatApiEndpoints.getActiveChatRun, {
    pathParams: { sessionCode },
    signal,
  });
}

export async function listChatUserInputRequests(
  sessionCode: string,
  signal?: AbortSignal,
): Promise<ChatUserInputRequest[]> {
  if (getServiceDataSource("chat") === "mock") return [];
  const requests = await requestApi<ChatUserInputRequest[]>(
    chatApiEndpoints.listChatUserInputRequests,
    { pathParams: { sessionCode }, signal },
  );
  return requests.map(normalizeChatUserInputRequest);
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

export async function steerChatMessage(
  input: SteerChatMessageInput,
  signal?: AbortSignal,
): Promise<ChatSteerResult> {
  if (getServiceDataSource("chat") === "mock") {
    const createdAt = new Date().toISOString();
    return {
      sessionCode: input.sessionCode,
      runCode: input.expectedRunCode,
      message: {
        actor: "User",
        attachments: input.attachments,
        channel: "MAIN_AGENT",
        content: input.content,
        createdAt,
        id: -Date.now(),
        messageCode: `MOCK-STEER-${Date.now()}`,
        role: "USER",
        runCode: input.expectedRunCode,
        sessionCode: input.sessionCode,
      },
    };
  }

  return requestApi<ChatSteerResult>(chatApiEndpoints.steerChatMessage, {
    body: input,
    signal,
  });
}

export async function stopChatRun(
  input: StopChatRunInput,
  signal?: AbortSignal,
): Promise<StopChatRunResult> {
  if (getServiceDataSource("chat") === "mock") {
    return {
      runCode: input.expectedRunCode,
      sessionCode: input.sessionCode,
    };
  }
  return requestApi<StopChatRunResult>(chatApiEndpoints.stopChatRun, {
    body: input,
    signal,
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
  signal?: AbortSignal,
): Promise<ChatSendResult> {
  if (getServiceDataSource("chat") === "mock") {
    const result = createMockChatSendResult(session, input);
    const assistant = result.messages.find((message) => message.role === "ASSISTANT");
    const runId = result.run.runCode;
    const messageId = assistant?.messageCode ?? `${runId}-assistant`;
    const startedAt = Date.now();
    const staleRunId = `${runId}-stale`;
    const firstRawReasoningContent = Array.from({ length: 18 }, (_, index) => `Raw reasoning stream segment ${index + 1}: validating raw provider reasoning stays out of the user card.`);
    const firstReasoningContentEvents: ChatAguiStreamEvent[] = firstRawReasoningContent.map((delta, index) => ({
      type: "REASONING_TEXT_CONTENT",
      threadId: session.sessionCode,
      runId,
      messageId: `${runId}-reasoning`,
      delta,
      timestamp: startedAt + 3 + index,
    }));
    firstReasoningContentEvents.push({
      type: "REASONING_MESSAGE_CONTENT",
      threadId: session.sessionCode,
      runId,
      messageId: `${runId}-reasoning`,
      delta: "Reviewing session context and available skills.",
      timestamp: startedAt + 3 + firstReasoningContentEvents.length,
    });
    const secondReasoningContent = "Summarizing tool results before replying.";
    const firstReasoningEndOffset = 3 + firstReasoningContentEvents.length;
    const toolCallDefinitions = [
      { name: "workspace.page", args: {}, result: "Listed workspaces" },
      { name: "app.page", args: {}, result: "Listed apps" },
      { name: "task.page", args: {}, result: "Listed tasks" },
      { name: "model.page", args: {}, result: "Listed models" },
      { name: "skill.page", args: {}, result: "Listed skills" },
      { name: "variable.page", args: {}, result: "Listed variables" },
    ];
    const toolCallEvents: ChatAguiStreamEvent[] = toolCallDefinitions.flatMap((tool, index) => {
      const toolTimestamp = startedAt + firstReasoningEndOffset + 2 + index * 4;
      return [
        {
          type: "TOOL_CALL_START",
          threadId: session.sessionCode,
          runId,
          toolCallId: `${runId}-tool-${index + 1}`,
          toolCallName: tool.name,
          toolCallArgs: tool.args,
          timestamp: toolTimestamp,
        },
        {
          type: "TOOL_CALL_ARGS",
          threadId: session.sessionCode,
          runId,
          toolCallId: `${runId}-tool-${index + 1}`,
          toolCallName: tool.name,
          toolCallArgs: tool.args,
          timestamp: toolTimestamp + 1,
        },
        {
          type: "TOOL_CALL_RESULT",
          threadId: session.sessionCode,
          runId,
          toolCallId: `${runId}-tool-${index + 1}`,
          toolCallName: tool.name,
          content: tool.result,
          timestamp: toolTimestamp + 2,
        },
        {
          type: "TOOL_CALL_END",
          threadId: session.sessionCode,
          runId,
          toolCallId: `${runId}-tool-${index + 1}`,
          timestamp: toolTimestamp + 3,
        },
      ];
    });
    const runningLogToolId = `${runId}-tool-log-scroll`;
    const shouldEmitLogScrollFixture = input.content.includes("scrolling tool log");
    const runningLogEvents: ChatAguiStreamEvent[] = shouldEmitLogScrollFixture
      ? [
          {
            type: "TOOL_CALL_START",
            threadId: session.sessionCode,
            runId,
            toolCallId: runningLogToolId,
            toolCallName: "bash_exec",
            toolCallArgs: { command: "bash", stdin: "for i in $(seq 1 40); do echo log-$i; done" },
            timestamp: startedAt + firstReasoningEndOffset + 2 + toolCallDefinitions.length * 4,
          },
          ...Array.from({ length: 40 }, (_, index) => ({
            type: "ACTIVITY_DELTA" as const,
            threadId: session.sessionCode,
            runId,
            toolCallId: runningLogToolId,
            toolCallName: "bash_exec",
            delta: [{ op: "add", path: "/logs/-", value: `stream log line ${index + 1}` }],
            timestamp: startedAt + firstReasoningEndOffset + 3 + toolCallDefinitions.length * 4 + index,
          })),
        ]
      : [];
    const unmatchedActivityTimestamp = startedAt + firstReasoningEndOffset + 2 + toolCallDefinitions.length * 4 + runningLogEvents.length;
    await new Promise((resolve) => { window.setTimeout(resolve, 120); });
    [
      { type: "RUN_STARTED", threadId: session.sessionCode, runId: staleRunId, timestamp: startedAt - 657_000 },
      { type: "REASONING_START", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 656_000 },
      { type: "REASONING_MESSAGE_START", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 655_000 },
      { type: "REASONING_MESSAGE_CONTENT", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, delta: "This stale run should not render.", timestamp: startedAt - 654_000 },
      { type: "REASONING_MESSAGE_END", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 653_000 },
      { type: "REASONING_END", threadId: session.sessionCode, runId: staleRunId, messageId: `${staleRunId}-reasoning`, timestamp: startedAt - 652_000 },
      { type: "RUN_FINISHED", threadId: session.sessionCode, runId: staleRunId, timestamp: startedAt - 651_000 },
      {
        type: "RUN_STARTED",
        threadId: session.sessionCode,
        runId,
        supportsSteer: input.agentCode === "main-agent",
        reasoningEffort: input.reasoningEffort ?? result.run.reasoningEffort ?? null,
        timestamp: startedAt,
      },
      { type: "REASONING_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 1 },
      { type: "REASONING_MESSAGE_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + 2 },
      ...firstReasoningContentEvents,
      { type: "REASONING_MESSAGE_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + firstReasoningEndOffset },
      { type: "REASONING_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: startedAt + firstReasoningEndOffset + 1 },
      ...toolCallEvents,
      ...runningLogEvents,
      { type: "ACTIVITY_DELTA", threadId: session.sessionCode, runId, toolCallId: `${runId}-unmatched-tool`, delta: [{ op: "add", path: "/logs/-", value: "This log belongs elsewhere" }], timestamp: unmatchedActivityTimestamp },
      { type: "REASONING_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: unmatchedActivityTimestamp + 1 },
      { type: "REASONING_MESSAGE_START", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: unmatchedActivityTimestamp + 2 },
      { type: "REASONING_MESSAGE_CONTENT", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, delta: secondReasoningContent, timestamp: unmatchedActivityTimestamp + 3 },
      { type: "REASONING_MESSAGE_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: unmatchedActivityTimestamp + 4 },
      { type: "REASONING_END", threadId: session.sessionCode, runId, messageId: `${runId}-reasoning`, timestamp: unmatchedActivityTimestamp + 5 },
      { type: "TEXT_MESSAGE_START", threadId: session.sessionCode, runId, messageId, role: "assistant", timestamp: unmatchedActivityTimestamp + 6 },
    ].forEach((event) => handlers.onAguiEvent?.(event));
    if (assistant?.content) {
      assistant.content.match(/.{1,18}/g)?.forEach((delta, index) => {
        handlers.onAguiEvent?.({
          type: "TEXT_MESSAGE_CONTENT",
          threadId: session.sessionCode,
          runId,
          messageId,
          delta,
          timestamp: unmatchedActivityTimestamp + 7 + index,
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

  return streamChatSse(
    chatApiEndpoints.streamChatMessage,
    session.sessionCode,
    input,
    handlers,
    signal,
  );
}

export function streamChatUserInputAnswer(
  session: ChatSession,
  requestCode: string,
  input: ChatUserInputAnswerInput,
  handlers: StreamChatMessageHandlers = {},
  signal?: AbortSignal,
): Promise<ChatSendResult> {
  if (getServiceDataSource("chat") === "mock") {
    return Promise.reject(new Error("User input answers are not available in mock mode"));
  }
  return streamChatSse(
    chatApiEndpoints.answerChatUserInput,
    session.sessionCode,
    input,
    handlers,
    signal,
    { requestCode },
  );
}

async function streamChatSse(
  endpoint: string,
  sessionCode: string,
  body: unknown,
  handlers: StreamChatMessageHandlers,
  signal?: AbortSignal,
  extraPathParams: Record<string, string> = {},
): Promise<ChatSendResult> {
  const { path } = parseEndpoint(endpoint);
  const headers = new Headers({ "Content-Type": "application/json" });
  applyCurrentLocaleHeader(headers);
  const accessToken = getApiAccessToken();
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);

  const response = await tracedFetch(buildApiUrl(path, { sessionCode, ...extraPathParams }), {
    body: JSON.stringify(body),
    credentials: "include",
    headers,
    method: "POST",
    signal,
  });

  if (!response.ok) {
    const payload = await response.json().catch(() => null);
    const message = isRecord(payload) && typeof payload.message === "string"
      ? payload.message
      : `POST ${response.url} failed with ${response.status}`;
    throw new ApiError(message, response.status, {
      code: isRecord(payload) && typeof payload.code === "string"
        ? payload.code
        : undefined,
      data: isRecord(payload) && "data" in payload ? payload.data : undefined,
    });
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
      throw new ApiError(
        payload.message ?? payload.code ?? "Chat stream failed",
        200,
        { code: payload.code },
      );
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
