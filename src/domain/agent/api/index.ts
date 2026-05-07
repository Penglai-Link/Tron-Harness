import { getMainAgentMessages } from "@/domain/agent/mock/data";

import { useServiceQuery } from "@/domain/service-config";
import type { AgentMessage } from "@/domain/agent/model/model";

export type {
  AgentMessage
} from "@/domain/agent/model/model";

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
