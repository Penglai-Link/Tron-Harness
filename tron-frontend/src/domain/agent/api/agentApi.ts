export const agentApiEndpoints = {
  listMainAgentMessages: "GET /api/v1/agent/main/messages",
  getMainAgentConfig: "GET /api/v1/agents/registry/main-agent",
  updateMainAgentConfig: "PATCH /api/v1/agents/registry/main-agent",
  listPlatformSystemAgents: "GET /api/v1/agents/registry/system",
  rescanPlatformSystemAgents: "POST /api/v1/agents/registry/system/rescan",
  enablePlatformSystemAgent: "POST /api/v1/agents/registry/system/{agentCode}/enable",
  disablePlatformSystemAgent: "POST /api/v1/agents/registry/system/{agentCode}/disable",
  listWorkspaceSystemAgents: "GET /api/v1/agents/workspaces/{workspaceCode}/system",
  enableWorkspaceSystemAgent: "POST /api/v1/agents/workspaces/{workspaceCode}/system/{agentCode}/enable",
  disableWorkspaceSystemAgent: "POST /api/v1/agents/workspaces/{workspaceCode}/system/{agentCode}/disable",
} as const;
