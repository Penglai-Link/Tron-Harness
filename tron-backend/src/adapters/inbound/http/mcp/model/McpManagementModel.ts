export type McpHealthStatus = "HEALTHY" | "UNHEALTHY" | "NEVER_CHECKED";
export type McpAuthKind = "none" | "bearer_static";
export type McpAccessMode = "disabled" | "all_enabled_servers" | "selected_servers";

export interface PlatformMcpGatewayConfig {
  gatewayEnabled: boolean;
  variablesEndpointEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface PlatformMcpServerRecord {
  serverCode: string;
  name: string;
  description: string;
  url: string;
  authKind: McpAuthKind;
  authPayload: Record<string, string>;
  headers: Record<string, string>;
  enabled: boolean;
  lastHealthStatus: McpHealthStatus;
  lastHealthMessage: string | null;
  lastHealthCheckedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface WorkspaceMcpServerRecord {
  serverCode: string;
  name: string;
  description: string;
  enabled: boolean;
  platformEnabled: boolean;
  effectiveEnabled: boolean;
  source: "platform";
  workspaceOverrideDisabled: boolean;
  lastHealthStatus: McpHealthStatus;
  lastHealthMessage: string | null;
  lastHealthCheckedAt: string | null;
  updatedAt: string;
}

export interface PlatformMcpGatewayPayload {
  gatewayEnabled: boolean;
  variablesEndpointEnabled: boolean;
}

export interface PlatformMcpServerPayload {
  serverCode: string;
  name: string;
  description: string;
  url: string;
  authKind: McpAuthKind;
  authPayload: Record<string, string>;
  headers: Record<string, string>;
}

export interface ListPlatformMcpServersQuery {
  search?: string;
  status?: "enabled" | "disabled" | "healthy" | "unhealthy" | "never_checked";
}
