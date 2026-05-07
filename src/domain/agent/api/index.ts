import { getMainAgentMessages } from "@/domain/agent/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
import type { AgentMessage } from "@/domain/agent/model/AgentModel";

export type {
  AgentMessage
} from "@/domain/agent/model/AgentModel";

export const agentApiEndpoints = {
  listMainAgentMessages: "GET /api/v1/agent/main/messages",
} as const;

export function useMainAgentMessages() {
  return useServiceQuery({
    domain: "agent",
    endpoint: agentApiEndpoints.listMainAgentMessages,
    mockData: () => getMainAgentMessages(),
    deps: [],
  });
}
