export interface CreateTerminalSocketTicketInput {
  appCode?: string | null;
  workspaceCode: string;
}

export interface TerminalSocketTicket {
  expiresAt: number;
  socketNamespace: string;
  socketPath: string;
  ticket: string;
}

export interface TerminalSocketConnection extends TerminalSocketTicket {
  socketUrl: string;
}

export interface GenerateTerminalCommandInput {
  modelCode?: string | null;
  prompt: string;
  workspaceCode: string;
}

export interface GenerateTerminalCommandResult {
  command: string;
  modelCode: string;
}

export interface TerminalAdapter {
  createSocketTicket(
    input: CreateTerminalSocketTicketInput,
    signal?: AbortSignal,
  ): Promise<TerminalSocketConnection>;
  generateCommand(
    input: GenerateTerminalCommandInput,
    signal?: AbortSignal,
  ): Promise<GenerateTerminalCommandResult>;
}

export function resolveTerminalSocketUrl(baseUrl: string | undefined, socketNamespace: string) {
  const fallbackOrigin = typeof window === "undefined" ? "http://localhost" : window.location.origin;
  const baseUrlObject = new URL(baseUrl ?? "", fallbackOrigin);
  const namespace = socketNamespace.trim();
  return new URL(namespace.startsWith("/") ? namespace : `/${namespace}`, baseUrlObject.origin).toString();
}
