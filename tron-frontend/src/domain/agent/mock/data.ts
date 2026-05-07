import type { AgentMessage } from "@/domain/agent/model/AgentModel";

export function getMainAgentMessages(): AgentMessage[] {
  return [
    { id: "m1", role: "user", body: "Assess the current load on the primary clusters and re-allocate resources if we are nearing the 80% threshold." },
    {
      id: "m2",
      role: "agent",
      body: "us-east-1-alpha is currently at 84% CPU utilization. I am preparing a scale-up sequence to add 3 standard nodes.",
    },
  ];
}
