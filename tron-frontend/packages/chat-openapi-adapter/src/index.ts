import {
  buildApiUrl,
  buildOpenApiSignedHeaders,
  createFetchTransport,
  FetchTransportError,
  normalizeChatActiveRunResult,
  normalizeChatMessage,
  normalizeChatSendResult,
  normalizeChatSession,
  normalizeChatSteerResult,
  normalizeChatUserInputRequest,
  type FetchTransportRequestContext,
  type BodyInitLike,
  type ChatAdapter,
  type ChatActiveRunResult,
  type ChatAttachmentUploadInput,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatMessage,
  type ChatMessageAttachment,
  type ChatClientConfig,
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
  randomHex,
  resolveTerminalSocketUrl,
  unwrapEnvelope,
} from "@tron/chat-core";

export interface OpenApiAdapterOptions extends Pick<ChatClientConfig, "appKey" | "baseUrl" | "openAppCode" | "userCode" | "userId"> {
  accessToken?: string | null;
  credentials?: RequestCredentials;
  getAccessToken?: () => string | null | undefined | Promise<string | null | undefined>;
  nonceFactory?: () => string;
  timestampFactory?: () => number;
}

interface AguiEventSnapshot {
  events: ChatAguiEvent[];
}

interface AguiProtocolEvent {
  data: ChatAguiStreamEvent;
  sseEvent: string;
}

function resolveUserCode(options: OpenApiAdapterOptions) {
  return options.userCode?.trim() || options.userId?.trim() || "";
}

async function openApiHeaders(
  options: OpenApiAdapterOptions,
  request: FetchTransportRequestContext,
) {
  return buildOpenApiSignedHeaders({
    openAppCode: options.openAppCode ?? "",
    appKey: options.appKey ?? "",
    body: request.body as BodyInitLike | undefined,
    method: request.method,
    nonce: options.nonceFactory?.(),
    path: request.path,
    rawQuery: request.rawQuery,
    timestamp: options.timestampFactory?.(),
    userCode: resolveUserCode(options),
  });
}

function attachmentCode(attachment: ChatMessageAttachment) {
  return attachment.resourceCode?.trim() || attachment.id.trim();
}

function escapeMultipartHeader(value: string) {
  return value.replace(/[\\"\r\n]/g, "_");
}

async function buildAttachmentMultipart(input: ChatAttachmentUploadInput) {
  const boundary = `----TronChatAttachment${randomHex(24)}`;
  const chunks: BlobPart[] = [];
  const appendField = (name: string, value: string) => {
    chunks.push(
      `--${boundary}\r\nContent-Disposition: form-data; name="${name}"\r\n\r\n${value}\r\n`,
    );
  };

  if (input.kind) appendField("kind", input.kind);
  if (input.summary?.trim()) appendField("summary", input.summary.trim());
  if (input.metadata && Object.keys(input.metadata).length) {
    appendField("metadata", JSON.stringify(input.metadata));
  }
  chunks.push(
    `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${escapeMultipartHeader(input.file.name)}"\r\nContent-Type: ${input.file.type || "application/octet-stream"}\r\n\r\n`,
    input.file,
    "\r\n",
    `--${boundary}--\r\n`,
  );

  return new Blob(chunks, { type: `multipart/form-data; boundary=${boundary}` });
}

async function openApiAttachmentRequest(
  options: OpenApiAdapterOptions,
  request: {
    body?: BodyInitLike;
    contentType?: string;
    method: string;
    path: string;
    signal?: AbortSignal;
  },
) {
  const url = buildApiUrl(options.baseUrl, request.path);
  const parsedUrl = new URL(url, typeof window === "undefined" ? "http://localhost" : window.location.origin);
  const headers = new Headers(await openApiHeaders(options, {
    body: request.body,
    method: request.method,
    path: parsedUrl.pathname,
    rawQuery: parsedUrl.search.startsWith("?") ? parsedUrl.search.slice(1) : parsedUrl.search,
    signal: request.signal,
    url,
  }));
  if (request.contentType) headers.set("Content-Type", request.contentType);

  const response = await fetch(url, {
    body: request.body instanceof Blob ? request.body : undefined,
    credentials: options.credentials ?? "omit",
    headers,
    method: request.method,
    signal: request.signal,
  });
  if (response.status < 200 || response.status >= 300) {
    const payload = await response.json().catch(() => null) as Record<string, unknown> | null;
    throw new FetchTransportError(
      typeof payload?.message === "string"
        ? payload.message
        : `${request.method} ${response.url} failed with ${response.status}`,
      {
        code: typeof payload?.code === "string" ? payload.code : undefined,
        data: payload?.data,
        status: response.status,
      },
    );
  }
  return response;
}

export function createOpenApiChatAdapter(options: OpenApiAdapterOptions = {}): ChatAdapter {
  const transport = createFetchTransport({
    baseUrl: options.baseUrl,
    credentials: options.credentials ?? "omit",
    getHeaders: (request) => openApiHeaders(options, request),
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
        path: "/open-api/v1/chats/sessions",
      }).then(normalizeChatSession);
    },
    deleteSession(sessionCode, signal) {
      return transport.requestJson<ChatSession>({
        method: "DELETE",
        path: "/open-api/v1/chats/sessions/{sessionCode}",
        pathParams: { sessionCode },
        signal,
      }).then(normalizeChatSession);
    },
    async downloadAttachment(session, attachment, signal) {
      const code = attachmentCode(attachment);
      if (!code) {
        throw new FetchTransportError("Chat attachment reference is missing", {
          code: "CHAT_ATTACHMENT_INVALID",
          status: 400,
        });
      }
      const response = await openApiAttachmentRequest(options, {
        method: "GET",
        path: `/open-api/v1/chats/sessions/${encodeURIComponent(session.sessionCode)}/attachments/${encodeURIComponent(code)}`,
        signal,
      });
      return response.blob();
    },
    getActiveRun(sessionCode, signal) {
      return transport.requestJson<ChatActiveRunResult>({
        path: "/open-api/v1/chats/sessions/get-active-run",
        queryParams: { sessionCode },
        signal,
      }).then(normalizeChatActiveRunResult);
    },
    async loadAguiEvents(sessionCode: string) {
      const payload = await transport.requestJson<AguiEventSnapshot>({
        path: "/open-api/v1/agents/agui/streams/{streamKey}/events",
        pathParams: { streamKey: sessionCode },
      });
      return payload.events ?? [];
    },
    loadMessages(sessionCode: string) {
      return transport.requestJson<ChatMessage[]>({
        path: "/open-api/v1/chats/sessions/{sessionCode}/messages",
        pathParams: { sessionCode },
      }).then((messages) => messages.map(normalizeChatMessage));
    },
    loadUserInputRequests(sessionCode, signal) {
      return transport.requestJson<ChatUserInputRequest[]>({
        path: "/open-api/v1/chats/sessions/{sessionCode}/user-input-requests",
        pathParams: { sessionCode },
        signal,
      }).then((requests) => requests.map(normalizeChatUserInputRequest));
    },
    async loadProtocolEvents(sessionCode: string) {
      const payload = await transport.requestJson<AguiProtocolEvent[]>({
        path: "/open-api/v1/agents/agui/streams/{streamKey}/protocol-events",
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
        path: "/open-api/v1/chats/sessions/steer-message",
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
        path: "/open-api/v1/chats/sessions/stop-run",
        signal,
      });
    },
    async uploadAttachment(session, input, signal) {
      const body = await buildAttachmentMultipart(input);
      const response = await openApiAttachmentRequest(options, {
        body,
        contentType: body.type,
        method: "POST",
        path: `/open-api/v1/chats/sessions/${encodeURIComponent(session.sessionCode)}/attachments`,
        signal,
      });
      return unwrapEnvelope<ChatMessageAttachment>(await response.json());
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
          if (type === "RUN_ERROR") {
            throw new FetchTransportError(
              String(payload.message ?? payload.code ?? "Chat stream failed"),
              {
                code: typeof payload.code === "string" ? payload.code : undefined,
                data: payload,
                status: 200,
              },
            );
          }
          return type === "RUN_FINISHED" && payload.result
            ? normalizeChatSendResult(payload.result as ChatSendResult)
            : null;
        },
        path: "/open-api/v1/chats/sessions/{sessionCode}/messages/stream",
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
          if (type === "RUN_ERROR") {
            throw new FetchTransportError(String(payload.message ?? payload.code ?? "Chat stream failed"), {
              code: typeof payload.code === "string" ? payload.code : undefined,
              data: payload,
              status: 200,
            });
          }
          return type === "RUN_FINISHED" && payload.result
            ? normalizeChatSendResult(payload.result as ChatSendResult)
            : null;
        },
        path: "/open-api/v1/chats/sessions/{sessionCode}/user-input-requests/{requestCode}/answers/stream",
        pathParams: { requestCode, sessionCode: session.sessionCode },
        signal,
      });
    },
  };
}

export function createOpenApiTerminalAdapter(
  options: OpenApiAdapterOptions = {},
): TerminalAdapter {
  const transport = createFetchTransport({
    baseUrl: options.baseUrl,
    credentials: options.credentials ?? "omit",
    getHeaders: (request) => openApiHeaders(options, request),
  });

  return {
    async createSocketTicket(input, signal) {
      const ticket = await transport.requestJson<TerminalSocketTicket>({
        body: {
          appCode: input.appCode ?? null,
          workspaceCode: input.workspaceCode,
        },
        method: "POST",
        path: "/open-api/v1/terminals/socket-tickets",
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
        path: "/open-api/v1/terminals/commands/generate",
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
