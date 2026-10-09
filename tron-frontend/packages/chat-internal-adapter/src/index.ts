import {
  buildApiUrl,
  createFetchTransport,
  FetchTransportError,
  normalizeChatActiveRunResult,
  normalizeChatMessage,
  normalizeChatSendResult,
  isChatSendResult,
  normalizeChatSession,
  normalizeChatSteerResult,
  normalizeChatUserInputRequest,
  type ChatAdapter,
  type ChatActiveRunResult,
  type ChatAttachmentUploadInput,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatMessage,
  type ChatMessageAttachment,
  type ChatSendResult,
  type ChatSession,
  type ChatSteerResult,
  type ChatStopResult,
  type ChatUserInputAnswerInput,
  type ChatUserInputRequest,
  type CreateChatSessionInput,
  type GenerateTerminalCommandInput,
  type GenerateTerminalCommandResult,
  type SendChatMessageInput,
  type TerminalAdapter,
  type TerminalSocketConnection,
  type TerminalSocketTicket,
  resolveTerminalSocketUrl,
} from "@tron/chat-core";

export interface InternalAdapterOptions {
  baseUrl?: string;
  credentials?: RequestCredentials;
  getAccessToken?: () => string | null | undefined | Promise<string | null | undefined>;
  getHeaders?: () => HeadersInit | undefined | Promise<HeadersInit | undefined>;
}

export interface InternalChatAdapterOptions extends InternalAdapterOptions {
  workspaceCode: string;
}

interface AguiEventSnapshot {
  replayIncomplete?: boolean;
  events: ChatAguiEvent[];
}

interface AguiProtocolEvent {
  data: ChatAguiStreamEvent;
  sseEvent: string;
}

async function requestHeaders(options: InternalAdapterOptions, workspaceCode?: string) {
  const headers = new Headers(await options.getHeaders?.());
  if (workspaceCode) headers.set("X-Workspace-Code", workspaceCode);
  const accessToken = await options.getAccessToken?.();
  if (accessToken) {
    headers.set("Authorization", `Bearer ${accessToken}`);
  }
  return headers;
}

export function createInternalChatAdapter(options: InternalChatAdapterOptions): ChatAdapter {
  const workspaceCode = options.workspaceCode.trim();
  const transport = createFetchTransport({
    baseUrl: options.baseUrl,
    credentials: options.credentials ?? "include",
    getHeaders: () => requestHeaders(options, workspaceCode),
  });

  return {
    capabilities: {
      supportsAbort: true,
      supportsActiveRun: true,
      supportsAgui: true,
      supportsProtocolReplay: true,
      supportsSteer: true,
      supportsStop: true,
      supportsStreaming: true,
      supportsUserInputRequests: true,
    },
    createSession(input: CreateChatSessionInput) {
      return transport.requestJson<ChatSession>({
        body: {
          agentCode: input.agentCode,
          appCode: input.appCode,
          sourceType: input.sourceType ?? null,
          taskCode: input.taskCode,
          title: input.title,
          workspaceCode: input.workspaceCode,
        },
        method: "POST",
        path: "/api/v1/chats/sessions",
      }).then(normalizeChatSession);
    },
    deleteSession(sessionCode, signal) {
      return transport.requestJson<ChatSession>({
        method: "DELETE",
        path: "/api/v1/chats/sessions/{sessionCode}",
        pathParams: { sessionCode },
        signal,
      }).then(normalizeChatSession);
    },
    async downloadAttachment(session, attachment, signal) {
      const attachmentCode = attachment.resourceCode?.trim() || attachment.id.trim();
      if (!attachmentCode) {
        throw new FetchTransportError("Chat attachment reference is missing", {
          code: "CHAT_ATTACHMENT_INVALID",
          status: 400,
        });
      }
      const url = buildApiUrl(
        options.baseUrl,
        "/api/v1/chats/sessions/{sessionCode}/attachments/{attachmentCode}",
        { attachmentCode, sessionCode: session.sessionCode },
      );
      const response = await fetch(url, {
        credentials: options.credentials ?? "include",
        headers: await requestHeaders(options, workspaceCode),
        method: "GET",
        signal,
      });
      if (!response.ok) {
        const payload = await response.json().catch(() => null) as Record<string, unknown> | null;
        throw new FetchTransportError(
          typeof payload?.message === "string"
            ? payload.message
            : `GET ${response.url} failed with ${response.status}`,
          {
            code: typeof payload?.code === "string" ? payload.code : undefined,
            data: payload?.data,
            status: response.status,
          },
        );
      }
      return response.blob();
    },
    getActiveRun(sessionCode, signal) {
      return transport.requestJson<ChatActiveRunResult>({
        path: "/api/v1/chats/sessions/get-active-run",
        queryParams: { sessionCode },
        signal,
      }).then(normalizeChatActiveRunResult);
    },
    async loadAguiEvents(sessionCode: string) {
      const payload = await transport.requestJson<AguiEventSnapshot>({
        path: "/api/v1/agents/agui/streams/{streamKey}/events",
        pathParams: { streamKey: sessionCode },
      });
      // These audit totals are auxiliary. A session-wide historical delivery
      // diagnostic cannot establish a failure in the latest native execution.
      // Native protocol errors and completion health carry run-specific notices.
      return payload.events ?? [];
    },
    loadMessages(sessionCode: string) {
      return transport.requestJson<ChatMessage[]>({
        path: "/api/v1/chats/sessions/{sessionCode}/messages",
        pathParams: { sessionCode },
      }).then((messages) => messages.map(normalizeChatMessage));
    },
    loadUserInputRequests(sessionCode, signal) {
      return transport.requestJson<ChatUserInputRequest[]>({
        path: "/api/v1/chats/sessions/{sessionCode}/user-input-requests",
        pathParams: { sessionCode },
        signal,
      }).then((requests) => requests.map(normalizeChatUserInputRequest));
    },
    async loadProtocolEvents(sessionCode: string) {
      const payload = await transport.requestJson<AguiProtocolEvent[]>({
        path: "/api/v1/agents/agui/streams/{streamKey}/protocol-events",
        pathParams: { streamKey: sessionCode },
      });
      return payload.map((event) => ({
        ...event.data,
        type: String(event.data?.type ?? event.sseEvent).toUpperCase(),
      }));
    },
    async steerMessage(session, input, context, signal) {
      const result = await transport.requestJson<ChatSteerResult>({
        body: {
          attachments: input.attachments ?? [],
          content: input.content,
          expectedRunCode: context.expectedRunCode,
          sessionCode: session.sessionCode,
        },
        method: "POST",
        path: "/api/v1/chats/sessions/steer-message",
        signal,
      });
      return normalizeChatSteerResult(result);
    },
    stopRun(session, context, signal) {
      return transport.requestJson<ChatStopResult>({
        body: {
          expectedRunCode: context.expectedRunCode,
          sessionCode: session.sessionCode,
        },
        method: "POST",
        path: "/api/v1/chats/sessions/stop-run",
        signal,
      });
    },
    uploadAttachment(
      session: ChatSession,
      input: ChatAttachmentUploadInput,
      signal?: AbortSignal,
    ) {
      const body = new FormData();
      if (input.kind) body.set("kind", input.kind);
      if (input.summary?.trim()) body.set("summary", input.summary.trim());
      if (input.metadata && Object.keys(input.metadata).length) {
        body.set("metadata", JSON.stringify(input.metadata));
      }
      body.set("file", input.file, input.file.name);
      return transport.requestJson<ChatMessageAttachment>({
        body,
        method: "POST",
        path: "/api/v1/chats/sessions/{sessionCode}/attachments",
        pathParams: { sessionCode: session.sessionCode },
        signal,
      });
    },
    streamMessage(session: ChatSession, input: SendChatMessageInput, handlers, signal) {
      return transport.streamSse<ChatSendResult>({
        body: {
          ...input,
          clientCapabilities: {
            ...input.clientCapabilities,
            userInputRequests: true,
          },
        },
        handlers: {
          onFrame: (eventName, payload) => {
            const type = String(payload.type ?? eventName).toUpperCase();
            const aguiEvent = { ...payload, type } as ChatAguiStreamEvent;
            handlers?.onAguiEvent?.(aguiEvent);
            if (type === "TEXT_MESSAGE_CONTENT" && typeof payload.delta === "string") {
              handlers?.onDelta?.(payload.delta);
            }
          },
        },
        onResult: (payload) => {
          const type = String(payload.type ?? "").toUpperCase();
          if (payload.productCompletion === true && payload.productCompletionError && typeof payload.productCompletionError === "object") {
            const error=payload.productCompletionError as Record<string,unknown>;
            throw new FetchTransportError(String(error.message ?? "Product completion is unavailable"), {code:typeof error.code === "string" ? error.code : "CHAT_STREAM_COMPLETION_UNAVAILABLE",data:payload,status:200});
          }
          if (type === "RUN_ERROR" && (payload.productCompletion === true || isChatSendResult(payload.result) || (!payload.metadata && !String(payload.code ?? "").startsWith("AGUI_")))) {
            throw new FetchTransportError(
              String(payload.message ?? payload.code ?? "Chat stream failed"),
              {
                code: typeof payload.code === "string" ? payload.code : undefined,
                data: payload,
                status: 200,
              },
            );
          }
          if (type !== "RUN_FINISHED") return null;
          if (isChatSendResult(payload.result)) return normalizeChatSendResult(payload.result);
          if (payload.productCompletion === true) {
            throw new FetchTransportError("Product completion result is invalid", {
              code: "CHAT_STREAM_COMPLETION_INVALID", status: 200,
            });
          }
          // Native child/continuation terminals carry SDK results and are
          // delivered above to onAguiEvent; only the product result settles SSE.
          return null;
        },
        path: "/api/v1/chats/sessions/{sessionCode}/messages/stream",
        pathParams: { sessionCode: session.sessionCode },
        signal,
      });
    },
    answerUserInputRequestStream(session, requestCode, input: ChatUserInputAnswerInput, handlers, signal) {
      return transport.streamSse<ChatSendResult>({
        body: {
          resume: input.resume ?? [{
            interruptId: requestCode,
            status: "resolved",
            payload: { answers: input.answers },
          }],
        },
        handlers: {
          onFrame: (eventName, payload) => {
            const type = String(payload.type ?? eventName).toUpperCase();
            const aguiEvent = { ...payload, type } as ChatAguiStreamEvent;
            handlers?.onAguiEvent?.(aguiEvent);
            if (type === "TEXT_MESSAGE_CONTENT" && typeof payload.delta === "string") {
              handlers?.onDelta?.(payload.delta);
            }
          },
        },
        onResult: (payload) => {
          const type = String(payload.type ?? "").toUpperCase();
          if (payload.productCompletion === true && payload.productCompletionError && typeof payload.productCompletionError === "object") {
            const error=payload.productCompletionError as Record<string,unknown>;
            throw new FetchTransportError(String(error.message ?? "Product completion is unavailable"), {code:typeof error.code === "string" ? error.code : "CHAT_STREAM_COMPLETION_UNAVAILABLE",data:payload,status:200});
          }
          if (type === "RUN_ERROR" && (payload.productCompletion === true || isChatSendResult(payload.result) || (!payload.metadata && !String(payload.code ?? "").startsWith("AGUI_")))) {
            throw new FetchTransportError(String(payload.message ?? payload.code ?? "Chat stream failed"), {
              code: typeof payload.code === "string" ? payload.code : undefined,
              data: payload,
              status: 200,
            });
          }
          if (type !== "RUN_FINISHED") return null;
          if (isChatSendResult(payload.result)) return normalizeChatSendResult(payload.result);
          if (payload.productCompletion === true) {
            throw new FetchTransportError("Product completion result is invalid", {
              code: "CHAT_STREAM_COMPLETION_INVALID", status: 200,
            });
          }
          // Native child/continuation terminals carry SDK results and are
          // delivered above to onAguiEvent; only the product result settles SSE.
          return null;
        },
        path: "/api/v1/chats/sessions/{sessionCode}/user-input-requests/{requestCode}/answers/stream",
        pathParams: { requestCode, sessionCode: session.sessionCode },
        signal,
      });
    },
  };
}

export function createInternalTerminalAdapter(
  options: InternalAdapterOptions = {},
): TerminalAdapter {
  const transport = createFetchTransport({
    baseUrl: options.baseUrl,
    credentials: options.credentials ?? "include",
    getHeaders: () => requestHeaders(options),
  });

  return {
    async createSocketTicket(input, signal) {
      const ticket = await transport.requestJson<TerminalSocketTicket>({
        body: {
          appCode: input.appCode ?? null,
          workspaceCode: input.workspaceCode,
        },
        method: "POST",
        path: "/api/v1/terminals/socket-tickets",
        signal,
      });
      return terminalSocketConnection(options.baseUrl, ticket);
    },
    generateCommand(input: GenerateTerminalCommandInput, signal) {
      return transport.requestJson<GenerateTerminalCommandResult>({
        body: {
          modelCode: input.modelCode ?? null,
          prompt: input.prompt,
          workspaceCode: input.workspaceCode,
        },
        method: "POST",
        path: "/api/v1/terminals/commands/generate",
        signal,
      });
    },
  };
}

function terminalSocketConnection(
  baseUrl: string | undefined,
  ticket: TerminalSocketTicket,
): TerminalSocketConnection {
  return {
    ...ticket,
    socketUrl: resolveTerminalSocketUrl(baseUrl, ticket.socketNamespace),
  };
}
