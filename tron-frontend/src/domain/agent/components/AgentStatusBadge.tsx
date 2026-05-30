import { getAgentStatusTone } from "@/domain/agent/constants/agentManagementOptions";
import type { AgentStatus } from "@/domain/agent/model";
import { Badge } from "@/shared/ui/Badge";

export function AgentStatusBadge({ status }: { status: AgentStatus }) {
  return (
    <Badge tone={getAgentStatusTone(status)}>
      {status}
    </Badge>
  );
}
