export interface TerminalSize {
  cols: number;
  rows: number;
}

export interface TerminalSession {
  currentDirectory: string;
}

export interface TerminalCommandOptions {
  interactive?: boolean;
  timeoutMs?: number;
}

export interface TerminalCommandResult {
  currentDirectory?: string;
}

export interface TransportDisposable {
  dispose(): void;
}

export interface TerminalTransport {
  cancel(): void;
  connect(): Promise<TerminalSession>;
  dispose(): void;
  execute(
    command: string,
    onOutput: (output: string) => void,
    options?: TerminalCommandOptions,
  ): Promise<TerminalCommandResult>;
  onSessionChange(
    listener: (session: TerminalSession) => void,
  ): TransportDisposable;
  resize(size: TerminalSize): void;
  write(data: string): void;
}

export interface AIStreamChunk {
  delta: string;
  requestId: string;
}

export interface AIStreamOptions {
  onChunk: (chunk: AIStreamChunk) => void;
  signal?: AbortSignal;
}

export interface AIStreamResult {
  requestId: string;
}

export interface AIRequestOptions {
  signal?: AbortSignal;
}

export interface AITransport {
  dispose?(): void;
  request(content: string, options?: AIRequestOptions): Promise<string>;
  stream(content: string, options: AIStreamOptions): Promise<AIStreamResult>;
}
