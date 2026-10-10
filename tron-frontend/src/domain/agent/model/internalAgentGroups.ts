import type { WorkspaceInternalAgentRecord } from "./AgentManagementModel";

export function internalAgentGroupName(agent: WorkspaceInternalAgentRecord, displayName: (agent: WorkspaceInternalAgentRecord) => string = (item) => item.name) {
  return agent.groupName || (agent.agentCode === "main-agent" ? displayName(agent) : "");
}

export function groupInternalAgents(records: WorkspaceInternalAgentRecord[], search: string, filter: string, displayName: (agent: WorkspaceInternalAgentRecord) => string = (agent) => agent.name) {
  const needle = search.trim().toLocaleLowerCase();
  const groups = new Map<string, WorkspaceInternalAgentRecord[]>();
  for (const agent of records) {
    const name = internalAgentGroupName(agent, displayName);
    if (filter !== "all" && (filter === "ungrouped" ? Boolean(name) : `group:${name}` !== filter)) continue;
    if (needle && ![agent.name, displayName(agent), agent.agentCode, name].some((value) => value.toLocaleLowerCase().includes(needle))) continue;
    const members = groups.get(name) ?? [];
    members.push(agent);
    groups.set(name, members);
  }
  return [...groups].sort(([left], [right]) => left === "" ? 1 : right === "" ? -1 : left.localeCompare(right))
    .map(([name, agents]) => ({ name, agents }));
}
