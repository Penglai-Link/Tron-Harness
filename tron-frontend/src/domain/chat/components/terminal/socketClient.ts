import { io, type Socket } from "socket.io-client";

export type TerminalMessageType =
  | "TEXT_MESSAGE_START"
  | "TEXT_MESSAGE_CONTENT"
  | "TEXT_MESSAGE_END"
  | "TEXT_MESSAGE_ERROR"
  | "TEXT_MESSAGE_SUGGESTION";

export interface TerminalMessage {
  currentDirectory?: string;
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
  "terminal:session": (response: unknown) => void;
}

interface ClientToServerEvents {
  "command:execute": (payload: CommandExecutePayload) => void;
  "command:cancel": (payload: { messageId: string }) => void;
}

interface PendingCommand {
  cancelled: boolean;
  onOutput: (output: string) => void;
  reject: (reason: Error) => void;
  resolve: (result: TerminalCommandResult) => void;
  timeoutId: number;
}

export interface TerminalSocketClientOptions {
  commandTimeoutMs?: number;
  onSessionChange?: (currentDirectory: string) => void;
}

export interface TerminalCommandResult {
  currentDirectory?: string;
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
  private readonly onSessionChange?: (currentDirectory: string) => void;
  private readonly pendingCommands = new Map<string, PendingCommand>();
  private readonly socket: Socket<ServerToClientEvents, ClientToServerEvents>;
  private connecting?: Promise<void>;
  private rejectConnecting?: (reason: Error) => void;
  private resolveConnecting?: () => void;
  private activeMessageId?: string;
  private destroyed = false;
  private sessionReady = false;

  constructor(url: string, options: TerminalSocketClientOptions = {}) {
    this.commandTimeoutMs =
      options.commandTimeoutMs ?? DEFAULT_COMMAND_TIMEOUT_MS;
    this.onSessionChange = options.onSessionChange;
    this.socket = io(url, {
      autoConnect: false,
      reconnection: true,
      timeout: 10_000,
      transports: ["websocket"],
    });

    this.socket.on("command:message", (response) =>
      this.handleMessage(response),
    );
    this.socket.on("terminal:session", (response) =>
      this.handleSession(response),
    );
    this.socket.on("disconnect", () => {
      this.sessionReady = false;
      const error = new Error("终端连接已断开");
      this.settleConnection(error);
      this.rejectAll(error);
    });
    this.socket.on("connect_error", () => {
      this.settleConnection(new Error("无法连接终端服务"));
    });
  }

  async execute(
    command: string,
    onOutput: (output: string) => void,
  ): Promise<TerminalCommandResult> {
    if (!command.trim()) return {};

    await this.connect();
    const messageId = createMessageId();

    return new Promise((resolve, reject) => {
      const timeoutId = window.setTimeout(() => {
        this.socket.emit("command:cancel", { messageId });
        this.rejectCommand(messageId, new Error("命令执行超时，请重试"));
      }, this.commandTimeoutMs);

      this.pendingCommands.set(messageId, {
        cancelled: false,
        onOutput,
        reject,
        resolve,
        timeoutId,
      });
      this.activeMessageId = messageId;
      this.socket.emit("command:execute", { command, messageId });
    });
  }

  cancelCurrent() {
    const messageId = this.activeMessageId;

    if (!messageId || !this.pendingCommands.has(messageId)) {
      return;
    }

    const pending = this.pendingCommands.get(messageId);
    if (!pending || pending.cancelled) return;

    pending.cancelled = true;
    this.socket.emit("command:cancel", { messageId });
  }

  destroy() {
    this.destroyed = true;
    this.sessionReady = false;
    const error = new Error("终端连接已关闭");
    this.settleConnection(error);
    this.rejectAll(error);
    this.socket.disconnect();
  }

  connect(): Promise<void> {
    if (this.destroyed) {
      return Promise.reject(new Error("终端已销毁"));
    }

    if (this.socket.connected && this.sessionReady) {
      return Promise.resolve();
    }

    if (this.connecting) {
      return this.connecting;
    }

    this.connecting = new Promise<void>((resolve, reject) => {
      this.resolveConnecting = resolve;
      this.rejectConnecting = reject;
    });

    if (!this.socket.connected) {
      this.socket.connect();
    }

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
        if (!pending.cancelled) {
          pending.onOutput(message.delta);
        }
        break;
      case "TEXT_MESSAGE_END":
        this.resolveCommand(message.messageId, {
          currentDirectory: message.currentDirectory,
        });
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

  private handleSession(response: unknown) {
    const session = parseTerminalSession(response);
    if (!session) return;

    if (session.code !== "0" || !session.data.currentDirectory) {
      this.sessionReady = false;
      const error = new Error(session.message || "终端会话不可用");
      this.settleConnection(error);
      this.rejectAll(error);
      return;
    }

    this.sessionReady = true;
    this.onSessionChange?.(session.data.currentDirectory);
    this.settleConnection();
  }

  private settleConnection(error?: Error) {
    const resolve = this.resolveConnecting;
    const reject = this.rejectConnecting;
    this.connecting = undefined;
    this.resolveConnecting = undefined;
    this.rejectConnecting = undefined;

    if (error) {
      reject?.(error);
    } else {
      resolve?.();
    }
  }

  private resolveCommand(
    messageId: string,
    result: TerminalCommandResult = {},
  ) {
    const pending = this.pendingCommands.get(messageId);
    if (!pending) return;

    window.clearTimeout(pending.timeoutId);
    this.pendingCommands.delete(messageId);
    if (this.activeMessageId === messageId) {
      this.activeMessageId = undefined;
    }
    pending.resolve(result);
  }

  private rejectCommand(messageId: string, error: Error) {
    const pending = this.pendingCommands.get(messageId);
    if (!pending) return;

    window.clearTimeout(pending.timeoutId);
    this.pendingCommands.delete(messageId);
    if (this.activeMessageId === messageId) {
      this.activeMessageId = undefined;
    }
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
    !TERMINAL_MESSAGE_TYPES.has(value.type as TerminalMessageType) ||
    (value.currentDirectory !== undefined &&
      (typeof value.currentDirectory !== "string" || !value.currentDirectory))
  ) {
    return undefined;
  }

  return value as unknown as TerminalMessage;
}

interface TerminalSessionResponse {
  code: string;
  data: { currentDirectory?: string };
  message: string;
}

function parseTerminalSession(value: unknown): TerminalSessionResponse | undefined {
  if (
    !isRecord(value) ||
    typeof value.code !== "string" ||
    typeof value.message !== "string" ||
    !isRecord(value.data) ||
    (value.data.currentDirectory !== undefined &&
      (typeof value.data.currentDirectory !== "string" ||
        !value.data.currentDirectory))
  ) {
    return undefined;
  }

  return value as unknown as TerminalSessionResponse;
}

function createMessageId() {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
