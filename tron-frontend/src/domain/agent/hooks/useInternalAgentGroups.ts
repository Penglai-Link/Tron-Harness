import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createInternalAgentGroup, deleteInternalAgentGroup, fetchInternalAgentGroups } from "../api/agentManagementApi";
import { createInternalAgentGroupMock, deleteInternalAgentGroupMock, listInternalAgentGroupsMock } from "../mock/managementData";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function useInternalAgentGroups(workspaceCode: string) {
  const client = useQueryClient();
  const source = getServiceDataSource("agent");
  const queryKey = ["internal-agent-groups", source, workspaceCode];
  const query = useQuery({
    queryKey,
    enabled: Boolean(workspaceCode),
    queryFn: () => source === "api" ? fetchInternalAgentGroups(workspaceCode) : Promise.resolve(listInternalAgentGroupsMock(workspaceCode)),
  });
  async function create(name: string) {
    const group = source === "api" ? await createInternalAgentGroup(workspaceCode, name) : createInternalAgentGroupMock(workspaceCode, name);
    client.setQueryData<string[]>(queryKey, (current = []) => [...new Set([...current, group.name])].sort());
    return group.name;
  }
  async function remove(name: string) {
    if (source === "api") await deleteInternalAgentGroup(workspaceCode, name);
    else deleteInternalAgentGroupMock(workspaceCode, name);
    client.setQueryData<string[]>(queryKey, (current = []) => current.filter((group) => group !== name));
  }
  return { ...query, groups: query.data ?? [], create, remove };
}
