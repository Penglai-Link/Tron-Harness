import {
  createFetchTransport,
  FetchTransportError,
  normalizeChatActiveRunResult,
  normalizeChatMessage,
  normalizeChatSendResult,
  normalizeChatSession,
  normalizeChatSteerResult,
  type ChatAdapter,
  type ChatActiveRunResult,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatMessage,
  type ChatSendResult,
  type ChatSession,
  type ChatSteerResult,
  type ChatStopResult,
  type CreateChatSessionInput,
  type SendChatMessageInput,
} from "@tron/chat-core";

interface InternalAdapterOptions {
  baseUrl?: string;
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

async function authHeaders(
  getAccessToken?: () => string | null | undefined | Promise<string | null | undefined>,
) {
  const accessToken = await getAccessToken?.();
  if (!accessToken) return undefined;
  return { Authorization: `Bearer ${accessToken}` };
}

export function createInternalChatAdapter(options: InternalAdapterOptions = {}): ChatAdapter {
  const transport = createFetchTransport({
    baseUrl: options.baseUrl,
    credentials: options.credentials ?? "include",
    getHeaders: () => authHeaders(options.getAccessToken),
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
        path: "/api/v1/chats/sessions",
      }).then(normalizeChatSession);
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
      return payload.events ?? [];
    },
    loadMessages(sessionCode: string) {
      return transport.requestJson<ChatMessage[]>({
        path: "/api/v1/chats/sessions/{sessionCode}/messages",
        pathParams: { sessionCode },
      }).then((messages) => messages.map(normalizeChatMessage));
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
        path: "/api/v1/chats/sessions/{sessionCode}/messages/stream",
        pathParams: { sessionCode: session.sessionCode },
        signal,
      });
    },
  };
}
