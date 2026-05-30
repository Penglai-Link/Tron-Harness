import type { AgentStatus } from "@/domain/agent/model";

export function getAgentStatusTone(status: AgentStatus) {
  switch (status) {
    case "Enabled":
      return "success" as const;
    case "Disabled":
      return "warning" as const;
    case "Unavailable":
      return "error" as const;
    default:
      return "neutral" as const;
  }
}
