import { expect, it } from "vitest";
import { availableAutomationAgents } from "./automationExecutionOptions";
import type { WorkspaceInternalAgentRecord } from "@/domain/agent/model";
it("does not recreate a missing or disabled main agent in execution choices", () => {
  expect(availableAutomationAgents([], [])).toEqual([]);
  const agents = [
    { agentCode: "main-agent", name: "Main", enabled: false },
    { agentCode: "other", name: "Other", enabled: true },
  ] as WorkspaceInternalAgentRecord[];
  expect(availableAutomationAgents([], agents).map(agent => agent.agentCode)).toEqual(["other"]);
});
