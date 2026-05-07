import {
  getChatMessages,
  getChatSessionGroups,
} from "@/domain/chat/mock/data";
import { useServiceQuery } from "@/domain/service-config";
import type {
  ChatSessionGroup,
  ChatMessage,
} from "@/domain/chat/model/model";

export type {
  ChatSessionGroup,
  ChatMessage
} from "@/domain/chat/model/model";

export const chatApiEndpoints = {
  listChatSessionGroups: "GET /api/v1/chats/session-groups",
  listChatMessages: "GET /api/v1/chats/{chatId}/messages?variant={variant}",
} as const;

export function useChatSessionGroups() {
  return useServiceQuery({
    domain: "chat",
    endpoint: chatApiEndpoints.listChatSessionGroups,
    mockData: () => getChatSessionGroups(),
    deps: [],
  });
}

export function useChatMessages(variant: "reasoning" | "execution", chatId = "8842-alpha") {
  return useServiceQuery({
    domain: "chat",
    endpoint: chatApiEndpoints.listChatMessages,
    mockData: () => getChatMessages(variant),
    deps: [variant, chatId],
    params: { variant, chatId },
  });
}
