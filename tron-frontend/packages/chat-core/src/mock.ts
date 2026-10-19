import {
  buildMockStreamingEvents,
} from "@eliteforge/agent-ui-chat-core";
import type {
  ChatAdapter,
  ChatAguiEvent,
  ChatAguiStreamEvent,
  ChatMessage,
  ChatSendResult,
  ChatSession,
  CreateChatSessionInput,
  SendChatMessageInput,
} from "./types";

function nowIso() {
  return new Date().toISOString();
}

export function createMockChatAdapter(): ChatAdapter {
  const sessions = new Map<string, ChatSession>();
  const messagesBySession = new Map<string, ChatMessage[]>();
  const protocolEventsBySession = new Map<string, ChatAguiStreamEvent[]>();

  return {
    capabilities: {
      supportsAbort: true,
      supportsActiveRun: false,
      supportsAgui: true,
      supportsProtocolReplay: true,
      supportsSteer: false,
      supportsStop: false,
      supportsStreaming: true,
    },
    async createSession(input: CreateChatSessionInput) {
      const sessionCode = `SESSION-${sessions.size + 1}`;
      const session: ChatSession = {
        agentCode: input.agentCode ?? "main-agent",
        appCode: input.appCode ?? null,
        createdAt: nowIso(),
        id: sessions.size + 1,
        lastRunCode: null,
        lastRunStatus: null,
        sessionCode,
        sourceType: input.sourceType ?? "MANUAL",
        status: "ACTIVE",
        systemAgentSessionId: null,
        taskCode: input.taskCode ?? null,
        title: input.title ?? "New chat",
        updatedAt: nowIso(),
        userCode: "mock-user",
        workspaceCode: input.workspaceCode,
      };
      sessions.set(sessionCode, session);
      messagesBySession.set(sessionCode, []);
      protocolEventsBySession.set(sessionCode, []);
      return session;
    },
    async deleteSession(sessionCode: string) {
      const session = sessions.get(sessionCode);
      if (!session) {
        throw new Error("Chat session was not found");
      }
      sessions.delete(sessionCode);
      messagesBySession.delete(sessionCode);
      protocolEventsBySession.delete(sessionCode);
      return session;
    },
    async loadAguiEvents() {
      return [] satisfies ChatAguiEvent[];
    },
    async loadMessages(sessionCode: string) {
      return [...(messagesBySession.get(sessionCode) ?? [])];
    },
    async loadProtocolEvents(sessionCode: string) {
      return [...(protocolEventsBySession.get(sessionCode) ?? [])];
    },
    async streamMessage(session, input, handlers, signal) {
      if (signal?.aborted) throw new DOMException("The operation was aborted.", "AbortError");
      const runCode = `RUN-${Date.now()}`;
      const assistantContent = `Echo from ${session.agentCode ?? "main-agent"}: ${input.content}`;
      const existing = messagesBySession.get(session.sessionCode) ?? [];
      const userMessage: ChatMessage = {
        actor: "User",
        channel: session.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        content: input.content,
        createdAt: nowIso(),
        id: existing.length + 1,
        messageCode: `${runCode}-USER`,
        role: "USER",
        runCode: null,
        sessionCode: session.sessionCode,
      };
      const assistantMessage: ChatMessage = {
        actor: session.agentCode ?? "Assistant",
        channel: session.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        content: assistantContent,
        createdAt: nowIso(),
        id: existing.length + 2,
        messageCode: `${runCode}-ASSISTANT`,
        role: "ASSISTANT",
        runCode,
        sessionCode: session.sessionCode,
      };
      const result: ChatSendResult = {
        messages: [userMessage, assistantMessage],
        run: {
          finalOutput: assistantContent,
          runCode,
          status: "COMPLETED",
          stepCount: 2,
          workspaceCode: session.workspaceCode,
        },
        session: {
          ...session,
          lastRunCode: runCode,
          lastRunStatus: "COMPLETED",
          updatedAt: nowIso(),
        },
      };
      const events: ChatAguiStreamEvent[] = buildMockStreamingEvents(session.sessionCode, runCode, assistantMessage.messageCode, assistantContent)
        .map(({ result: _result, ...event }) => event);
      for (const event of events) {
        if (signal?.aborted) throw new DOMException("The operation was aborted.", "AbortError");
        handlers?.onAguiEvent?.(event);
        if (event.type === "TEXT_MESSAGE_CONTENT" && typeof event.delta === "string") {
          handlers?.onDelta?.(event.delta);
        }
      }
      sessions.set(session.sessionCode, result.session);
      messagesBySession.set(session.sessionCode, [...existing, ...result.messages]);
      protocolEventsBySession.set(session.sessionCode, [...(protocolEventsBySession.get(session.sessionCode) ?? []), ...events]);
      return result;
    },
  };
}
