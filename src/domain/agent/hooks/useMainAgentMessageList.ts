import { getMainAgentMessages } from "@/domain/agent/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { agentApiEndpoints } from "@/domain/agent/api";

export function useMainAgentMessageList() {
  return useServiceQuery({
    domain: "agent",
    endpoint: agentApiEndpoints.listMainAgentMessages,
    mockData: () => getMainAgentMessages(),
    deps: [],
  });
}
