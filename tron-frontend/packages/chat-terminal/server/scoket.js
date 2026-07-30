import "reflect-metadata";

import * as pty from "node-pty";
import {
  SubscribeMessage,
  WebSocketGateway,
  WebSocketServer,
} from "@nestjs/websockets";

import { MessageType, createMessage, resolveMessageId } from "./message.js";

export const TERMINAL_MESSAGE_EVENT = "command:message";

export class TerminalGateway {
  constructor(options = {}) {
    this.now = options.now ?? Date.now;
    this.spawn = options.spawn ?? pty.spawn;
    this.terminalsByClient = new Map();
  }

  handleConnection(client) {
    this.terminalsByClient.set(client.id, new Map());
  }

  handleDisconnect(client) {
    const terminals = this.terminalsByClient.get(client.id);

    if (terminals) {
      for (const terminal of terminals.values()) {
        killTerminal(terminal);
      }
    }

    this.terminalsByClient.delete(client.id);
  }

  executeCommand(client, payload) {
    const messageId = resolveMessageId(payload?.messageId);
    const command = typeof payload?.command === "string" ? payload.command.trim() : "";

    if (!command) {
      this.emit(client, MessageType.ERROR, "命令不能为空", messageId);
      return;
    }

    const terminals = this.getClientTerminals(client);
    if (terminals.has(messageId)) {
      this.emit(client, MessageType.ERROR, "消息ID对应的命令正在执行", messageId);
      return;
    }

    const [shell, args] = getShellCommand(command);
    let terminal;

    try {
      terminal = this.spawn(shell, args, {
        cols: 80,
        cwd: process.cwd(),
        env: process.env,
        name: "xterm-256color",
        rows: 24,
      });
    } catch (error) {
      this.emit(
        client,
        MessageType.ERROR,
        getErrorMessage(error, "命令无法启动"),
        messageId,
      );
      return;
    }

    terminals.set(messageId, terminal);
    this.emit(client, MessageType.START, "", messageId);

    terminal.onData((delta) => {
      this.emit(client, MessageType.CONTENT, delta, messageId);
    });
    terminal.onExit(() => {
      terminals.delete(messageId);
      this.emit(client, MessageType.END, "", messageId);
    });
  }

  cancelCommand(client, payload) {
    if (typeof payload?.messageId !== "string" || !payload.messageId.trim()) {
      this.emit(client, MessageType.ERROR, "取消命令时必须提供消息ID");
      return;
    }

    const messageId = payload.messageId.trim();
    const terminal = this.terminalsByClient.get(client.id)?.get(messageId);

    if (!terminal) {
      this.emit(client, MessageType.ERROR, "未找到正在执行的命令", messageId);
      return;
    }

    killTerminal(terminal);
  }

  emit(client, type, delta, messageId) {
    client.emit(
      TERMINAL_MESSAGE_EVENT,
      createMessage({ delta, messageId, now: this.now, type }),
    );
  }

  getClientTerminals(client) {
    let terminals = this.terminalsByClient.get(client.id);

    if (!terminals) {
      terminals = new Map();
      this.terminalsByClient.set(client.id, terminals);
    }

    return terminals;
  }
}

function getShellCommand(command) {
  if (process.platform === "win32") {
    return [process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", command]];
  }

  return [process.env.SHELL ?? "/bin/sh", ["-lc", command]];
}

function killTerminal(terminal) {
  try {
    terminal.kill();
  } catch {
    // The process may already have exited between lookup and cancellation.
  }
}

function getErrorMessage(error, fallback) {
  return error instanceof Error && error.message ? error.message : fallback;
}

WebSocketGateway({
  cors: { origin: process.env.TERMINAL_DEMO_ORIGIN ?? "*" },
  transports: ["websocket"],
})(TerminalGateway);
WebSocketServer()(TerminalGateway.prototype, "server");

SubscribeMessage("command:execute")(
  TerminalGateway.prototype,
  "executeCommand",
  Object.getOwnPropertyDescriptor(TerminalGateway.prototype, "executeCommand"),
);
SubscribeMessage("command:cancel")(
  TerminalGateway.prototype,
  "cancelCommand",
  Object.getOwnPropertyDescriptor(TerminalGateway.prototype, "cancelCommand"),
);
