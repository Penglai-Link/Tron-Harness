import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { fetchAgentWorkspaceGrants, saveAgentWorkspaceGrants } from "../api/agentWorkspaceGrants";
import { listAgentWorkspaceGrantsMock, saveAgentWorkspaceGrantsMock } from "../mock/agentWorkspaceGrantsMock";

export async function loadAgentWorkspaceGrants(code: string, signal?: AbortSignal) {
  return getServiceDataSource("agent") === "api"
    ? fetchAgentWorkspaceGrants(code, signal)
    : listAgentWorkspaceGrantsMock(code);
}

export async function persistAgentWorkspaceGrants(code: string, workspaceCodes: string[]) {
  return getServiceDataSource("agent") === "api"
    ? saveAgentWorkspaceGrants(code, workspaceCodes)
    : saveAgentWorkspaceGrantsMock(code, workspaceCodes);
}
