import "reflect-metadata";

import { homedir } from "node:os";

import * as pty from "node-pty";
import {
  SubscribeMessage,
  WebSocketGateway,
  WebSocketServer,
} from "@nestjs/websockets";

import {
  prepareShell,
  subshellBootstrap,
} from "./shellIntegration.js";

export const TERMINAL_SESSION_EVENT = "terminal:session";
export const TERMINAL_OUTPUT_EVENT = "terminal:output";
export const TERMINAL_EXIT_EVENT = "terminal:exit";
export const TERMINAL_INPUT_EVENT = "terminal:input";
export const TERMINAL_RESIZE_EVENT = "terminal:resize";
export const TERMINAL_SIGNAL_EVENT = "terminal:signal";

const MAX_INPUT_BYTES = 64 * 1024;
const SIGNALS = { SIGINT: "\x03", SIGKILL: undefined };

/**
 * Dev reference implementation of the terminal protocol the Rust backend speaks.
 *
 * The PTY is authoritative: keystrokes are forwarded verbatim and every byte the
 * shell produces is streamed back untouched. Command boundaries are not framed
 * here at all — the shell reports them itself through the OSC 133 markers the
 * integration script installs, which is what keeps command blocks and AI routing
 * working inside `ssh`, `vim` and other nested programs.
 */
export class TerminalGateway {
  constructor(options = {}) {
    this.cwd = options.cwd ?? homedir();
    this.env = options.env ?? process.env;
    this.platform = options.platform ?? process.platform;
    this.shell = options.shell;
    this.spawn = options.spawn ?? pty.spawn;
    this.prepareShell = options.prepareShell ?? prepareShell;
    this.subshellBootstrap = options.subshellBootstrap ?? subshellBootstrap;
    this.sessionsByClient = new Map();
  }

  handleConnection(client) {
    this.cleanupSession(client.id, { kill: true });

    if (this.platform === "win32") {
      client.emit(TERMINAL_SESSION_EVENT, {
        error: "Windows 暂不支持 shell integration，请在 WSL 中运行",
      });
      return;
    }

    const shell = this.prepareShell({ env: this.env, shell: this.shell });
    let terminal;

    try {
      terminal = this.spawn(shell.command, shell.args, {
        cols: 80,
        cwd: this.cwd,
        encoding: "utf8",
        env: {
          ...shell.env,
          COLORTERM: "truecolor",
          TERM: "xterm-256color",
          TRON_SHELL_INTEGRATION: "1",
        },
        name: "xterm-256color",
        rows: 24,
      });
    } catch (error) {
      shell.cleanup?.();
      client.emit(TERMINAL_SESSION_EVENT, {
        error: getErrorMessage(error, "终端会话无法启动"),
      });
      return;
    }

    const session = {
      client,
      disposed: false,
      shell,
      suppressOutputUntil: 0,
      terminal,
    };
    this.sessionsByClient.set(client.id, session);

    session.dataDisposable = terminal.onData((data) => {
      if (session.disposed) {
        return;
      }
      if (Date.now() >= session.suppressOutputUntil) {
        client.emit(TERMINAL_OUTPUT_EVENT, { data });
      }
    });
    session.exitDisposable = terminal.onExit((event) => {
      this.handleTerminalExit(session, event);
    });

    client.emit(TERMINAL_SESSION_EVENT, {
      currentDirectory: this.cwd,
      shell: shell.kind,
      // A shell without precmd/preexec hooks still runs; the client just
      // degrades to a plain terminal instead of showing blocks.
      supportsBlocks: shell.supportsBlocks,
      subshellBootstrap: this.subshellBootstrap(),
    });
  }

  handleDisconnect(client) {
    this.cleanupSession(client.id, { kill: true });
  }

  writeInput(client, payload) {
    const data = typeof payload?.data === "string" ? payload.data : "";
    const session = this.sessionsByClient.get(client.id);

    if (
      !session ||
      session.disposed ||
      (!data && payload?.clearBuffer !== true) ||
      Buffer.byteLength(data, "utf8") > MAX_INPUT_BYTES ||
      (typeof payload?.inputLine === "string" &&
        Buffer.byteLength(payload.inputLine, "utf8") > MAX_INPUT_BYTES)
    ) {
      return;
    }

    try {
      if (payload?.clearBuffer === true) {
        session.suppressOutputUntil = Date.now() + 250;
      }
      session.terminal.write(`${payload?.clearBuffer === true ? "\x03" : ""}${data}`);
    } catch {
      // The shell exited between the keystroke and this write; onExit reports it.
    }
  }

  sendSignal(client, payload) {
    const session = this.sessionsByClient.get(client.id);
    if (!session || session.disposed) {
      return;
    }

    if (payload?.signal === "SIGKILL") {
      killTerminal(session.terminal);
      return;
    }

    const sequence = SIGNALS[payload?.signal];
    if (!sequence) {
      return;
    }

    try {
      // Written into the PTY rather than sent to the child, so the foreground
      // job of the moment receives it — including one running inside `ssh`.
      session.terminal.write(sequence);
    } catch {
      // Same as writeInput: the shell is already gone.
    }
  }

  resizeTerminal(client, payload) {
    const cols = Number(payload?.cols);
    const rows = Number(payload?.rows);
    const session = this.sessionsByClient.get(client.id);

    if (
      !session ||
      session.disposed ||
      !isTerminalDimension(cols) ||
      !isTerminalDimension(rows)
    ) {
      return;
    }

    try {
      session.terminal.resize(cols, rows);
    } catch {
      // A resize racing the shell's exit is not worth reporting.
    }
  }

  handleTerminalExit(session, event) {
    if (session.disposed) {
      return;
    }

    const exitCode = typeof event?.exitCode === "number" ? event.exitCode : null;
    session.client.emit(TERMINAL_EXIT_EVENT, { exitCode });
    this.cleanupSession(session.client.id, { kill: false });
  }

  cleanupSession(clientId, { kill }) {
    const session = this.sessionsByClient.get(clientId);
    if (!session || session.disposed) {
      return;
    }

    session.disposed = true;
    this.sessionsByClient.delete(clientId);
    session.dataDisposable?.dispose?.();
    session.exitDisposable?.dispose?.();
    if (kill) {
      killTerminal(session.terminal);
    }
    session.shell?.cleanup?.();
  }
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

SubscribeMessage(TERMINAL_INPUT_EVENT)(
  TerminalGateway.prototype,
  "writeInput",
  Object.getOwnPropertyDescriptor(TerminalGateway.prototype, "writeInput"),
);
SubscribeMessage(TERMINAL_SIGNAL_EVENT)(
  TerminalGateway.prototype,
  "sendSignal",
  Object.getOwnPropertyDescriptor(TerminalGateway.prototype, "sendSignal"),
);
SubscribeMessage(TERMINAL_RESIZE_EVENT)(
  TerminalGateway.prototype,
  "resizeTerminal",
  Object.getOwnPropertyDescriptor(TerminalGateway.prototype, "resizeTerminal"),
);
