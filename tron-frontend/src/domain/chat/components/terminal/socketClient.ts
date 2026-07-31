import { io, type Socket } from "socket.io-client";

export type TerminalMessageType =
  | "TEXT_MESSAGE_START"
  | "TEXT_MESSAGE_CONTENT"
  | "TEXT_MESSAGE_END"
  | "TEXT_MESSAGE_ERROR"
  | "TEXT_MESSAGE_SUGGESTION";

export interface TerminalMessage {
  delta: string;
  messageId: string;
  timestamp: number;
  type: TerminalMessageType;
}

interface CommandExecutePayload {
  messageId: string;
  command: string;
}

interface ServerToClientEvents {
  "command:message": (response: unknown) => void;
}

interface ClientToServerEvents {
  "command:execute": (payload: CommandExecutePayload) => void;
  "command:cancel": (payload: { messageId: string }) => void;
}

interface PendingCommand {
  onOutput: (output: string) => void;
  reject: (reason: Error) => void;
  resolve: () => void;
  timeoutId: number;
}

export interface TerminalSocketClientOptions {
  commandTimeoutMs?: number;
}

const DEFAULT_COMMAND_TIMEOUT_MS = 60_000;
const TERMINAL_MESSAGE_TYPES = new Set<TerminalMessageType>([
  "TEXT_MESSAGE_START",
  "TEXT_MESSAGE_CONTENT",
  "TEXT_MESSAGE_END",
  "TEXT_MESSAGE_ERROR",
  "TEXT_MESSAGE_SUGGESTION",
]);

export class TerminalSocketClient {
  private readonly commandTimeoutMs: number;
  private readonly pendingCommands = new Map<string, PendingCommand>();
  private readonly socket: Socket<ServerToClientEvents, ClientToServerEvents>;
  private connecting?: Promise<void>;
  private destroyed = false;

  constructor(url: string, options: TerminalSocketClientOptions = {}) {
    this.commandTimeoutMs =
      options.commandTimeoutMs ?? DEFAULT_COMMAND_TIMEOUT_MS;
    this.socket = io(url, {
      autoConnect: false,
      reconnection: true,
      timeout: 10_000,
      transports: ["websocket"],
    });

    this.socket.on("command:message", (response) =>
      this.handleMessage(response),
    );
    this.socket.on("disconnect", () => {
      this.rejectAll(new Error("终端连接已断开"));
    });
  }

  async execute(
    command: string,
    onOutput: (output: string) => void,
  ): Promise<void> {
    if (!command.trim()) return;

    await this.connect();
    const messageId = createMessageId();

    return new Promise((resolve, reject) => {
      const timeoutId = window.setTimeout(() => {
        this.socket.emit("command:cancel", { messageId });
        this.rejectCommand(messageId, new Error("命令执行超时，请重试"));
      }, this.commandTimeoutMs);

      this.pendingCommands.set(messageId, {
        onOutput,
        reject,
        resolve,
        timeoutId,
      });
      this.socket.emit("command:execute", { command, messageId });
    });
  }

  destroy() {
    this.destroyed = true;
    this.rejectAll(new Error("终端连接已关闭"));
    this.socket.disconnect();
  }

  connect(): Promise<void> {
    if (this.destroyed) {
      return Promise.reject(new Error("终端已销毁"));
    }

    if (this.socket.connected) {
      return Promise.resolve();
    }

    if (this.connecting) {
      return this.connecting;
    }

    this.connecting = new Promise<void>((resolve, reject) => {
      const handleConnect = () => {
        this.socket.off("connect_error", handleConnectError);
        resolve();
      };
      const handleConnectError = () => {
        this.socket.off("connect", handleConnect);
        reject(new Error("无法连接终端服务"));
      };

      this.socket.once("connect", handleConnect);
      this.socket.once("connect_error", handleConnectError);
      this.socket.connect();
    }).finally(() => {
      this.connecting = undefined;
    });

    return this.connecting;
  }

  private handleMessage(response: unknown) {
    const message = parseTerminalMessage(response);
    if (!message) {
      console.warn("忽略格式错误的终端消息", response);
      return;
    }

    const pending = this.pendingCommands.get(message.messageId);
    if (!pending) return;

    switch (message.type) {
      case "TEXT_MESSAGE_CONTENT":
        pending.onOutput(message.delta);
        break;
      case "TEXT_MESSAGE_END":
        this.resolveCommand(message.messageId);
        break;
      case "TEXT_MESSAGE_ERROR":
        this.rejectCommand(
          message.messageId,
          new Error(message.delta || "命令执行失败"),
        );
        break;
      default:
        break;
    }
  }

  private resolveCommand(messageId: string) {
    const pending = this.pendingCommands.get(messageId);
    if (!pending) return;

    window.clearTimeout(pending.timeoutId);
    this.pendingCommands.delete(messageId);
    pending.resolve();
  }

  private rejectCommand(messageId: string, error: Error) {
    const pending = this.pendingCommands.get(messageId);
    if (!pending) return;

    window.clearTimeout(pending.timeoutId);
    this.pendingCommands.delete(messageId);
    pending.reject(error);
  }

  private rejectAll(error: Error) {
    for (const messageId of this.pendingCommands.keys()) {
      this.rejectCommand(messageId, error);
    }
  }
}

export function parseTerminalMessage(
  value: unknown,
): TerminalMessage | undefined {
  if (
    !isRecord(value) ||
    typeof value.delta !== "string" ||
    typeof value.messageId !== "string" ||
    !value.messageId ||
    typeof value.timestamp !== "number" ||
    !Number.isFinite(value.timestamp) ||
    typeof value.type !== "string" ||
    !TERMINAL_MESSAGE_TYPES.has(value.type as TerminalMessageType)
  ) {
    return undefined;
  }

  return value as unknown as TerminalMessage;
}

function createMessageId() {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
