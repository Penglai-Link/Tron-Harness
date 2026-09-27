import type { ChatAdapter, ChatAguiStreamEvent } from "@tron/chat-core";
import { createMockChatSession, deleteMockChatSession, getChatAguiEvents, getChatMessages } from "./data";
import { streamChatMessage } from "../hooks/useChatMessageListBySession";

// The shell, context sidebar and public Chat UI must share the same domain data.
const protocolEvents = new Map<string, ChatAguiStreamEvent[]>();

export function createTronMockChatAdapter(): ChatAdapter {
  return {
    capabilities: {
      supportsAbort: true, supportsAgui: true, supportsProtocolReplay: true,
      supportsStreaming: true, supportsActiveRun: false, supportsSteer: false, supportsStop: false,
    },
    async createSession(input) { return createMockChatSession(input); },
    async deleteSession(id) {
      const result = deleteMockChatSession(id);
      protocolEvents.delete(id);
      return result;
    },
    async loadMessages(id) { return getChatMessages(id); },
    async loadAguiEvents(id) { return getChatAguiEvents(id); },
    async loadProtocolEvents(id) { return [...(protocolEvents.get(id) ?? [])]; },
    async streamMessage(session, input, handlers, signal) {
      if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
      return streamChatMessage({ ...session, sourceType: session.sourceType ?? undefined }, input, {
        onDelta: handlers?.onDelta,
        onAguiEvent(event) {
          if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
          const events = protocolEvents.get(session.sessionCode) ?? [];
          protocolEvents.set(session.sessionCode, [...events, event]);
          handlers?.onAguiEvent?.(event);
        },
      }, signal);
    },
  };
}
