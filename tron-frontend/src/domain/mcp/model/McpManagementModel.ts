export type McpHealthStatus = "HEALTHY" | "UNHEALTHY" | "NEVER_CHECKED";
export type McpAuthKind = "none" | "bearer_static";
export type McpServerTransportKind = "streamable_http" | "stdio";
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
  transportKind: McpServerTransportKind;
  url: string;
  authKind: McpAuthKind;
  authPayload: Record<string, string> | null;
  headers: Record<string, string>;
  headersVariableRefs: Record<string, string>;
  command: string;
  args: string[];
  envVariableRefs: Record<string, string>;
  envPassthroughRefs: string[];
  workingDirectory: string | null;
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

export interface PlatformMcpServerDefinitionPayload {
  name: string;
  description: string;
  transportKind: McpServerTransportKind;
  url: string;
  authKind: McpAuthKind;
  authPayload: Record<string, string> | null;
  headers: Record<string, string>;
  headersVariableRefs: Record<string, string>;
  command: string;
  args: string[];
  envVariableRefs: Record<string, string>;
  envPassthroughRefs: string[];
  workingDirectory: string;
}

export interface PlatformMcpServerPayload extends PlatformMcpServerDefinitionPayload {
  serverCode: string;
}

export interface ListPlatformMcpServersQuery {
  search?: string;
  status?: "enabled" | "disabled" | "healthy" | "unhealthy" | "never_checked";
}

export interface VerifyPlatformMcpServerDraftPayload {
  workspaceCode?: string;
  definition: PlatformMcpServerPayload;
}

export interface VerifyPlatformMcpServerPayload {
  workspaceCode?: string;
  definition?: PlatformMcpServerDefinitionPayload;
}
