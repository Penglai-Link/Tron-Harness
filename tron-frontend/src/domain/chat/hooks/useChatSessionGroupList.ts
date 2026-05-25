import { getChatSessionGroups } from "@/domain/chat/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { chatApiEndpoints } from "@/domain/chat/api";

export function useChatSessionGroupList() {
  return useServiceQuery({
    apiInitialData: () => [],
    domain: "chat",
    endpoint: chatApiEndpoints.listChatSessionGroups,
    mockData: () => getChatSessionGroups(),
    deps: [],
  });
}
