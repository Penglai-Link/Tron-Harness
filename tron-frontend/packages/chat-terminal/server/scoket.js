import "reflect-metadata";

import { randomUUID } from "node:crypto";
import path from "node:path";
import { homedir } from 'os';

import * as pty from "node-pty";
import {
  SubscribeMessage,
  WebSocketGateway,
  WebSocketServer,
} from "@nestjs/websockets";

import { MessageType, createMessage, resolveMessageId } from "./message.js";

export const TERMINAL_MESSAGE_EVENT = "command:message";
export const TERMINAL_SESSION_EVENT = "terminal:session";
export const TERMINAL_INPUT_EVENT = "command:input";
export const TERMINAL_RESIZE_EVENT = "terminal:resize";

const RECORD_SEPARATOR = "\x1e";
const UNIT_SEPARATOR = "\x1f";
const SESSION_READY_PREFIX = `${RECORD_SEPARATOR}TRON_SESSION_READY:`;
const COMMAND_END_PREFIX = `${RECORD_SEPARATOR}TRON_COMMAND_END:`;
const MAX_BOOTSTRAP_BUFFER_LENGTH = 64 * 1024;

export class TerminalGateway {
  constructor(options = {}) {
    this.createMessageId = options.createMessageId ?? randomUUID;
    this.createToken = options.createToken ?? randomUUID;
    this.cwd = homedir();
    this.env = options.env ?? process.env;
    this.now = options.now ?? Date.now;
    this.platform = options.platform ?? process.platform;
    this.shell = options.shell;
    this.spawn = options.spawn ?? pty.spawn;
    this.sessionsByClient = new Map();
  }

  handleConnection(client) {
    this.cleanupSession(client.id, { kill: true });

    const shell = getShellConfiguration({
      env: this.env,
      platform: this.platform,
      shell: this.shell,
    });
    let terminal;

    try {
      terminal = this.spawn(shell.command, shell.args, {
        cols: 80,
        cwd: this.cwd,
        env: shell.env,
        name: "xterm-256color",
        rows: 24,
      });
    } catch (error) {
      this.emitSessionError(client, getErrorMessage(error, "终端会话无法启动"));
      return;
    }

    const session = {
      activeCommand: undefined,
      bootstrapBuffer: "",
      client,
      currentDirectory: this.cwd,
      dataDisposable: undefined,
      disposed: false,
      exitDisposable: undefined,
      ready: false,
      shell,
      terminal,
    };

    this.sessionsByClient.set(client.id, session);
    session.dataDisposable = terminal.onData((data) => {
      this.handleTerminalData(session, data);
    });
    session.exitDisposable = terminal.onExit(() => {
      this.handleTerminalExit(session);
    });

    try {
      terminal.write(shell.bootstrapCommand);
    } catch (error) {
      this.emitSessionError(
        client,
        getErrorMessage(error, "终端会话无法初始化"),
      );
      this.cleanupSession(client.id, { kill: true });
    }
  }

  handleDisconnect(client) {
    this.cleanupSession(client.id, { kill: true });
  }

  executeCommand(client, payload) {
    const messageId = resolveMessageId(payload?.messageId);
    const command =
      typeof payload?.command === "string" ? payload.command.trim() : "";
    const interactive = payload?.interactive === true;

    if (!command) {
      this.emit(client, MessageType.ERROR, "命令不能为空", messageId);
      return;
    }

    const session = this.sessionsByClient.get(client.id);
    if (!session || session.disposed) {
      this.emit(client, MessageType.ERROR, "终端会话不存在", messageId);
      return;
    }
    if (!session.ready) {
      this.emit(client, MessageType.ERROR, "终端会话尚未就绪", messageId);
      return;
    }
    if (session.activeCommand) {
      this.emit(client, MessageType.ERROR, "已有命令正在执行", messageId);
      return;
    }

    const token = this.createToken();
    session.activeCommand = {
      cancelling: false,
      interactive,
      markerPrefix: `${COMMAND_END_PREFIX}${token}:`,
      messageId,
      outputBuffer: "",
    };
    this.emit(client, MessageType.START, "", messageId);

    try {
      session.terminal.write(session.shell.frameCommand(command, token));
    } catch (error) {
      session.activeCommand = undefined;
      this.emit(
        client,
        MessageType.ERROR,
        getErrorMessage(error, "命令无法执行"),
        messageId,
      );
    }
  }

  cancelCommand(client, payload) {
    if (typeof payload?.messageId !== "string" || !payload.messageId.trim()) {
      this.emit(client, MessageType.ERROR, "取消命令时必须提供消息ID");
      return;
    }

    const messageId = payload.messageId.trim();
    const session = this.sessionsByClient.get(client.id);
    const activeCommand = session?.activeCommand;

    if (!session || !activeCommand || activeCommand.messageId !== messageId) {
      this.emit(client, MessageType.ERROR, "未找到正在执行的命令", messageId);
      return;
    }
    if (activeCommand.cancelling) {
      return;
    }

    activeCommand.cancelling = true;
    try {
      session.terminal.write(session.shell.interruptInput);
    } catch (error) {
      this.emit(
        client,
        MessageType.ERROR,
        getErrorMessage(error, "命令无法中断"),
        messageId,
      );
    }
  }

  writeCommandInput(client, payload) {
    const messageId =
      typeof payload?.messageId === "string" ? payload.messageId.trim() : "";
    const data = typeof payload?.data === "string" ? payload.data : "";
    const session = this.sessionsByClient.get(client.id);
    const activeCommand = session?.activeCommand;

    if (
      !session ||
      session.disposed ||
      !activeCommand?.interactive ||
      activeCommand.messageId !== messageId ||
      !data
    ) {
      return;
    }

    try {
      session.terminal.write(data);
    } catch (error) {
      this.emit(
        client,
        MessageType.ERROR,
        getErrorMessage(error, "命令输入无法发送"),
        messageId,
      );
    }
  }

  resizeTerminal(client, payload) {
    const cols = Number(payload?.cols);
    const rows = Number(payload?.rows);
    const session = this.sessionsByClient.get(client.id);

    if (
      !session ||
      session.disposed ||
      !session.ready ||
      !isTerminalDimension(cols) ||
      !isTerminalDimension(rows)
    ) {
      return;
    }

    try {
      session.terminal.resize(cols, rows);
    } catch (error) {
      this.emitSessionError(
        client,
        getErrorMessage(error, "终端窗口大小无法调整"),
      );
    }
  }

  emit(client, type, delta, messageId, currentDirectory) {
    client.emit(
      TERMINAL_MESSAGE_EVENT,
      createMessage({
        currentDirectory,
        delta,
        messageId,
        now: this.now,
        type,
      }),
    );
  }

  emitSessionReady(session) {
    session.client.emit(TERMINAL_SESSION_EVENT, {
      ...createMessage({
        currentDirectory: session.currentDirectory,
        delta: "",
        messageId: this.createMessageId(),
        now: this.now,
        type: MessageType.CONTENT,
      }),
    });
  }

  emitSessionError(client, message) {
    client.emit(TERMINAL_SESSION_EVENT, {
      ...createMessage({
        delta: message,
        messageId: this.createMessageId(),
        now: this.now,
        type: MessageType.ERROR,
      }),
    });
  }

  handleTerminalData(session, data) {
    if (session.disposed) {
      return;
    }

    if (!session.ready) {
      this.handleBootstrapData(session, data);
      return;
    }

    const activeCommand = session.activeCommand;
    if (!activeCommand) {
      return;
    }

    activeCommand.outputBuffer += data;
    this.flushCommandOutput(session, activeCommand);
  }

  handleBootstrapData(session, data) {
    session.bootstrapBuffer += data;
    const marker = readDelimitedMarker(
      session.bootstrapBuffer,
      SESSION_READY_PREFIX,
    );

    if (!marker) {
      if (session.bootstrapBuffer.length > MAX_BOOTSTRAP_BUFFER_LENGTH) {
        session.bootstrapBuffer = session.bootstrapBuffer.slice(
          -MAX_BOOTSTRAP_BUFFER_LENGTH,
        );
      }
      return;
    }

    session.bootstrapBuffer = "";
    session.currentDirectory = marker.payload || this.cwd;
    session.ready = true;
    this.emitSessionReady(session);
  }

  flushCommandOutput(session, activeCommand) {
    const buffer = activeCommand.outputBuffer;
    const markerStart = buffer.indexOf(activeCommand.markerPrefix);

    if (markerStart >= 0) {
      const markerEnd = buffer.indexOf(UNIT_SEPARATOR, markerStart);
      const content = buffer.slice(0, markerStart);
      if (content) {
        this.emit(
          session.client,
          MessageType.CONTENT,
          content,
          activeCommand.messageId,
        );
      }

      if (markerEnd < 0) {
        activeCommand.outputBuffer = buffer.slice(markerStart);
        return;
      }

      const payload = buffer.slice(
        markerStart + activeCommand.markerPrefix.length,
        markerEnd,
      );
      const separatorIndex = payload.indexOf(":");
      const currentDirectory =
        separatorIndex >= 0 ? payload.slice(separatorIndex + 1) : "";

      if (currentDirectory) {
        session.currentDirectory = currentDirectory;
      }
      session.activeCommand = undefined;
      this.emit(
        session.client,
        MessageType.END,
        "",
        activeCommand.messageId,
        session.currentDirectory,
      );
      return;
    }

    const retainedLength = longestSuffixPrefixLength(
      buffer,
      activeCommand.markerPrefix,
    );
    const contentEnd = buffer.length - retainedLength;
    const content = buffer.slice(0, contentEnd);
    activeCommand.outputBuffer = buffer.slice(contentEnd);

    if (content) {
      this.emit(
        session.client,
        MessageType.CONTENT,
        content,
        activeCommand.messageId,
      );
    }
  }

  handleTerminalExit(session) {
    if (session.disposed) {
      return;
    }

    const activeMessageId = session.activeCommand?.messageId;
    if (activeMessageId) {
      this.emit(
        session.client,
        MessageType.ERROR,
        "终端会话已结束",
        activeMessageId,
      );
    }
    this.emitSessionError(session.client, "终端会话已结束");
    this.cleanupSession(session.client.id, { kill: false });
  }

  cleanupSession(clientId, { kill }) {
    const session = this.sessionsByClient.get(clientId);
    if (!session || session.disposed) {
      return;
    }

    session.disposed = true;
    session.activeCommand = undefined;
    this.sessionsByClient.delete(clientId);
    session.dataDisposable?.dispose?.();
    session.exitDisposable?.dispose?.();
    if (kill) {
      killTerminal(session.terminal);
    }
  }
}

function getShellConfiguration({ env, platform, shell }) {
  if (platform === "win32") {
    return {
      args: ["/d", "/q", "/v:on"],
      bootstrapCommand: `@echo off\r\n@echo ${SESSION_READY_PREFIX}%CD%${UNIT_SEPARATOR}\r\n`,
      command: shell ?? env.ComSpec ?? "cmd.exe",
      env: { ...env, PROMPT: "$S" },
      frameCommand: (command, token) =>
        `${command} & @echo ${COMMAND_END_PREFIX}${token}:!ERRORLEVEL!:!CD!${UNIT_SEPARATOR}\r\n`,
      interruptInput: "\x03",
    };
  }

  const command = shell ?? env.SHELL ?? "/bin/sh";
  return {
    args: getPosixShellArgs(command),
    bootstrapCommand: getPosixBootstrapCommand(command),
    command,
    env,
    frameCommand: (input, token) =>
      `stty echo; eval ${quotePosixShell(input)}; tron_command_status=$?; stty -echo; printf '\\036TRON_COMMAND_END:${token}:%s:%s\\037\\n' "$tron_command_status" "$PWD"\n`,
    interruptInput: "\x03",
  };
}

function quotePosixShell(value) {
  return `'${value.replaceAll("'", `'\\''`)}'`;
}

function getPosixBootstrapCommand(shell) {
  const name = path.basename(shell);
  const setup = ["stty -echo", "PROMPT=''", "RPROMPT=''", "PS1=''", "PS2=''"];

  if (name === "zsh") {
    setup.push(
      "unsetopt zle prompt_cr prompt_sp",
      "precmd_functions=()",
      "preexec_functions=()",
      "unfunction precmd preexec 2>/dev/null || true",
    );
  } else if (name === "bash") {
    setup.push("set +o emacs", "set +o vi");
  }

  return `${setup.join("; ")}; printf '\\036TRON_SESSION_READY:%s\\037\\n' "$PWD"\n`;
}

function getPosixShellArgs(shell) {
  const name = path.basename(shell);
  if (name === "bash") {
    return ["--login", "-i"];
  }
  if (name === "zsh") {
    return ["-l", "-i"];
  }
  return ["-i"];
}

function readDelimitedMarker(buffer, prefix) {
  const start = buffer.indexOf(prefix);
  if (start < 0) {
    return undefined;
  }
  const end = buffer.indexOf(UNIT_SEPARATOR, start + prefix.length);
  if (end < 0) {
    return undefined;
  }
  return {
    payload: buffer.slice(start + prefix.length, end),
  };
}

function longestSuffixPrefixLength(value, prefix) {
  const maxLength = Math.min(value.length, prefix.length - 1);
  for (let length = maxLength; length > 0; length -= 1) {
    if (value.endsWith(prefix.slice(0, length))) {
      return length;
    }
  }
  return 0;
}

function killTerminal(terminal) {
  try {
    terminal.kill();
  } catch {
    // The process may already have exited during disconnect cleanup.
  }
}

function getErrorMessage(error, fallback) {
  return error instanceof Error && error.message ? error.message : fallback;
}

function isTerminalDimension(value) {
  return Number.isInteger(value) && value > 0 && value <= 1_000;
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
SubscribeMessage(TERMINAL_INPUT_EVENT)(
  TerminalGateway.prototype,
  "writeCommandInput",
  Object.getOwnPropertyDescriptor(
    TerminalGateway.prototype,
    "writeCommandInput",
  ),
);
SubscribeMessage(TERMINAL_RESIZE_EVENT)(
  TerminalGateway.prototype,
  "resizeTerminal",
  Object.getOwnPropertyDescriptor(
    TerminalGateway.prototype,
    "resizeTerminal",
  ),
);
