import { getChatMessages } from "@/domain/chat/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { chatApiEndpoints } from "@/domain/chat/api";

export function useChatMessages(variant: "reasoning" | "execution", chatId = "8842-alpha") {
  return useServiceQuery({
    domain: "chat",
    endpoint: chatApiEndpoints.listChatMessages,
    mockData: () => getChatMessages(variant),
    deps: [variant, chatId],
    params: { variant, chatId },
  });
}
