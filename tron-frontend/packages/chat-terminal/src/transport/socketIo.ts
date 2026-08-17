import {
  io,
  type ManagerOptions,
  type Socket,
  type SocketOptions,
} from "socket.io-client";

import {
  createMessageId,
  parseTerminalMessage,
  parseTerminalSession,
} from "./protocol";
import type {
  TerminalCommandOptions,
  TerminalCommandResult,
  TerminalSession,
  TerminalSize,
  TerminalTransport,
  TransportDisposable,
} from "./types";

interface CommandExecutePayload {
  command: string;
  interactive?: true;
  messageId: string;
}

interface ServerToClientEvents {
  "command:message": (response: unknown) => void;
  "terminal:session": (response: unknown) => void;
}

interface ClientToServerEvents {
  "command:cancel": (payload: { messageId: string }) => void;
  "command:execute": (payload: CommandExecutePayload) => void;
  "command:input": (payload: { data: string; messageId: string }) => void;
  "terminal:resize": (payload: TerminalSize) => void;
}

interface PendingCommand {
  cancelled: boolean;
  interactive: boolean;
  onOutput: (output: string) => void;
  reject: (reason: Error) => void;
  resolve: (result: TerminalCommandResult) => void;
  timeoutId?: number;
}

export interface TerminalTransportOptions {
  url: string;
  commandTimeoutMs?: number;
  socketOptions?: Partial<ManagerOptions & SocketOptions>;
}

type TerminalTransportRuntimeOptions = Omit<TerminalTransportOptions, "url">;

const DEFAULT_COMMAND_TIMEOUT_MS = 60_000;

export class SocketIoTerminalTransport implements TerminalTransport {
  private readonly commandTimeoutMs: number;
  private readonly pendingCommands = new Map<string, PendingCommand>();
  private readonly sessionListeners = new Set<
    (session: TerminalSession) => void
  >();
  private readonly socket: Socket<ServerToClientEvents, ClientToServerEvents>;
  private connecting?: Promise<TerminalSession>;
  private rejectConnecting?: (reason: Error) => void;
  private resolveConnecting?: (session: TerminalSession) => void;
  private activeMessageId?: string;
  private currentSession?: TerminalSession;
  private latestSize?: TerminalSize;
  private destroyed = false;

  constructor(url: string, options: TerminalTransportRuntimeOptions = {}) {
    this.commandTimeoutMs =
      options.commandTimeoutMs ?? DEFAULT_COMMAND_TIMEOUT_MS;
    this.socket = io(url, {
      reconnection: true,
      timeout: 10_000,
      transports: ["websocket"],
      ...options.socketOptions,
      autoConnect: false,
    });

    this.socket.on("command:message", (response) =>
      this.handleMessage(response),
    );
    this.socket.on("terminal:session", (response) =>
      this.handleSession(response),
    );
    this.socket.on("disconnect", () => {
      this.currentSession = undefined;
      const error = new Error("终端连接已断开");
      this.settleConnection(undefined, error);
      this.rejectAll(error);
    });
    this.socket.on("connect_error", () => {
      this.settleConnection(undefined, new Error("无法连接终端服务"));
    });
  }

  connect(): Promise<TerminalSession> {
    if (this.destroyed) {
      return Promise.reject(new Error("终端已销毁"));
    }

    if (this.socket.connected && this.currentSession) {
      return Promise.resolve(this.currentSession);
    }

    if (this.connecting) {
      return this.connecting;
    }

    this.connecting = new Promise<TerminalSession>((resolve, reject) => {
      this.resolveConnecting = resolve;
      this.rejectConnecting = reject;
    });

    if (!this.socket.connected) {
      this.socket.connect();
    }

    return this.connecting;
  }

  async execute(
    command: string,
    onOutput: (output: string) => void,
    options: TerminalCommandOptions = {},
  ): Promise<TerminalCommandResult> {
    if (!command.trim()) return {};

    await this.connect();
    const messageId = createMessageId();
    const interactive = options.interactive === true;

    return new Promise((resolve, reject) => {
      const timeoutMs = interactive
        ? 0
        : (options.timeoutMs ?? this.commandTimeoutMs);
      const timeoutId =
        timeoutMs > 0
          ? window.setTimeout(() => {
              this.socket.emit("command:cancel", { messageId });
              this.rejectCommand(messageId, new Error("命令执行超时，请重试"));
            }, timeoutMs)
          : undefined;

      this.pendingCommands.set(messageId, {
        cancelled: false,
        interactive,
        onOutput,
        reject,
        resolve,
        timeoutId,
      });
      this.activeMessageId = messageId;
      this.socket.emit("command:execute", {
        command,
        messageId,
        ...(interactive ? { interactive: true as const } : {}),
      });
    });
  }

  cancel() {
    const messageId = this.activeMessageId;
    const pending = messageId
      ? this.pendingCommands.get(messageId)
      : undefined;

    if (!messageId || !pending || pending.cancelled) {
      return;
    }

    pending.cancelled = true;
    this.socket.emit("command:cancel", { messageId });
  }

  write(data: string) {
    const messageId = this.activeMessageId;
    const pending = messageId
      ? this.pendingCommands.get(messageId)
      : undefined;

    if (!data || !messageId || !pending?.interactive || pending.cancelled) {
      return;
    }

    this.socket.emit("command:input", { data, messageId });
  }

  resize(size: TerminalSize) {
    if (!isTerminalSize(size)) {
      return;
    }

    this.latestSize = size;
    this.emitTerminalSize();
  }

  onSessionChange(
    listener: (session: TerminalSession) => void,
  ): TransportDisposable {
    this.sessionListeners.add(listener);
    if (this.currentSession) {
      listener(this.currentSession);
    }

    return {
      dispose: () => {
        this.sessionListeners.delete(listener);
      },
    };
  }

  dispose() {
    this.destroyed = true;
    this.currentSession = undefined;
    const error = new Error("终端连接已关闭");
    this.settleConnection(undefined, error);
    this.rejectAll(error);
    this.sessionListeners.clear();
    this.socket.disconnect();
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
    if ("error" in session) {
      this.currentSession = undefined;
      const error = new Error(session.error);
      this.settleConnection(undefined, error);
      this.rejectAll(error);
      return;
    }

    this.currentSession = { currentDirectory: session.currentDirectory };
    for (const listener of this.sessionListeners) {
      listener(this.currentSession);
    }
    this.emitTerminalSize();
    this.settleConnection(this.currentSession);
  }

  private emitTerminalSize() {
    if (!this.latestSize || !this.socket.connected || !this.currentSession) {
      return;
    }

    this.socket.emit("terminal:resize", this.latestSize);
  }

  private settleConnection(session?: TerminalSession, error?: Error) {
    const resolve = this.resolveConnecting;
    const reject = this.rejectConnecting;
    this.connecting = undefined;
    this.resolveConnecting = undefined;
    this.rejectConnecting = undefined;

    if (error) {
      reject?.(error);
    } else if (session) {
      resolve?.(session);
    }
  }

  private resolveCommand(
    messageId: string,
    result: TerminalCommandResult = {},
  ) {
    const pending = this.pendingCommands.get(messageId);
    if (!pending) return;

    if (pending.timeoutId !== undefined) {
      window.clearTimeout(pending.timeoutId);
    }
    this.pendingCommands.delete(messageId);
    if (this.activeMessageId === messageId) {
      this.activeMessageId = undefined;
    }
    pending.resolve(result);
  }

  private rejectCommand(messageId: string, error: Error) {
    const pending = this.pendingCommands.get(messageId);
    if (!pending) return;

    if (pending.timeoutId !== undefined) {
      window.clearTimeout(pending.timeoutId);
    }
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

export function createTerminalTransport(options: TerminalTransportOptions) {
  return new SocketIoTerminalTransport(options.url, {
    commandTimeoutMs: options.commandTimeoutMs,
    socketOptions: options.socketOptions,
  });
}

function isTerminalSize(value: TerminalSize) {
  return (
    Number.isInteger(value.cols) &&
    value.cols > 0 &&
    Number.isInteger(value.rows) &&
    value.rows > 0
  );
}
