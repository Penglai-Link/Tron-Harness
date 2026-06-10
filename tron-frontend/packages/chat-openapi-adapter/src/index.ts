import {
  createFetchTransport,
  normalizeChatMessage,
  normalizeChatSendResult,
  normalizeChatSession,
  type ChatAdapter,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatMessage,
  type ChatClientConfig,
  type ChatSendResult,
  type ChatSession,
  type CreateChatSessionInput,
  type SendChatMessageInput,
} from "@tron/chat-core";

interface OpenApiAdapterOptions extends Pick<ChatClientConfig, "appId" | "appKey" | "baseUrl" | "userId"> {
  accessToken?: string | null;
  credentials?: RequestCredentials;
  getAccessToken?: () => string | null | undefined | Promise<string | null | undefined>;
}

interface AguiEventSnapshot {
  events: ChatAguiEvent[];
}

interface AguiProtocolEvent {
  data: ChatAguiStreamEvent;
  sseEvent: string;
}

async function openApiHeaders(options: OpenApiAdapterOptions) {
  const headers = new Headers();
  const accessToken = options.accessToken ?? await options.getAccessToken?.();
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);
  if (options.appId) headers.set("X-Tron-App-Id", options.appId);
  if (options.appKey) headers.set("X-Tron-App-Key", options.appKey);
  if (options.userId) headers.set("X-Tron-User-Id", options.userId);
  return headers;
}

export function createOpenApiChatAdapter(options: OpenApiAdapterOptions = {}): ChatAdapter {
  const transport = createFetchTransport({
    baseUrl: options.baseUrl,
    credentials: options.credentials ?? "omit",
    getHeaders: () => openApiHeaders(options),
  });

  return {
    capabilities: {
      supportsAbort: true,
      supportsAgui: true,
      supportsProtocolReplay: true,
      supportsStreaming: true,
    },
    createSession(input: CreateChatSessionInput) {
      return transport.requestJson<ChatSession>({
        body: {
          agentCode: input.agentCode,
          appCode: input.appCode,
          taskCode: input.taskCode,
          title: input.title,
          workspaceCode: input.workspaceCode,
        },
        method: "POST",
        path: "/open-api/v1/chats/sessions",
      }).then(normalizeChatSession);
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
    streamMessage(session: ChatSession, input: SendChatMessageInput, handlers, signal) {
      return transport.streamSse<ChatSendResult>({
        body: input,
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
            throw new Error(String(payload.message ?? payload.code ?? "Chat stream failed"));
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
  };
}
