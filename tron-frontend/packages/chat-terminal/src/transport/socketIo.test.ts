// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

const socketHarness = vi.hoisted(() => {
  const handlers = new Map<string, (...args: unknown[]) => void>();
  const socket = {
    connected: true,
    connect: vi.fn(),
    disconnect: vi.fn(),
    emit: vi.fn(),
    on: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
      handlers.set(event, handler);
    }),
  };

  const io = vi.fn(() => socket);

  return { handlers, io, socket };
});

vi.mock("socket.io-client", () => ({
  io: socketHarness.io,
}));

import {
  SocketIoTerminalTransport,
  createTerminalTransport,
} from "./socketIo";

describe("SocketIoTerminalTransport", () => {
  beforeEach(() => {
    socketHarness.handlers.clear();
    socketHarness.io.mockClear();
    socketHarness.socket.emit.mockClear();
    socketHarness.socket.on.mockClear();
    vi.stubGlobal("crypto", {
      randomUUID: vi.fn().mockReturnValue("message-1"),
    });
  });

  it("waits for the PTY session and publishes its directory", async () => {
    const transport = new SocketIoTerminalTransport("http://terminal.test");
    const onSessionChange = vi.fn();
    transport.onSessionChange(onSessionChange);
    const connection = transport.connect();

    readySession("/workspace/project");

    await expect(connection).resolves.toEqual({
      currentDirectory: "/workspace/project",
    });
    expect(onSessionChange).toHaveBeenCalledWith({
      currentDirectory: "/workspace/project",
    });
  });

  it("creates a terminal transport from an options object", () => {
    const transport = createTerminalTransport({
      url: "http://terminal.test",
      commandTimeoutMs: 30_000,
      socketOptions: {
        auth: { token: "token" },
      },
    });

    expect(transport).toBeInstanceOf(SocketIoTerminalTransport);
  });

  it("creates an isolated socket connection for every terminal transport", () => {
    new SocketIoTerminalTransport("http://terminal.test", {
      socketOptions: { auth: { token: "token" } },
    });

    expect(socketHarness.io).toHaveBeenCalledWith(
      "http://terminal.test",
      expect.objectContaining({
        autoConnect: false,
        forceNew: true,
        multiplex: false,
      }),
    );
  });

  it("rejects connection when the session envelope contains an error", async () => {
    const transport = new SocketIoTerminalTransport("http://terminal.test");
    const connection = transport.connect();

    socketHarness.handlers.get("terminal:session")?.({
      delta: "终端会话无法启动",
      messageId: "session-1",
      timestamp: 1,
      type: "TEXT_MESSAGE_ERROR",
    });

    await expect(connection).rejects.toThrow("终端会话无法启动");
  });

  it("executes commands and resolves the resulting directory", async () => {
    const transport = new SocketIoTerminalTransport("http://terminal.test");
    readySession();
    const onOutput = vi.fn();
    const execution = transport.execute("pwd", onOutput);
    await Promise.resolve();

    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:execute", {
      command: "pwd",
      messageId: "message-1",
    });

    commandMessage("TEXT_MESSAGE_CONTENT", "/workspace\r\n");
    commandMessage("TEXT_MESSAGE_END", "", "/workspace");

    expect(onOutput).toHaveBeenCalledWith("/workspace\r\n");
    await expect(execution).resolves.toEqual({
      currentDirectory: "/workspace",
    });
  });

  it("forwards interactive input without creating a timeout", async () => {
    const timeoutSpy = vi.spyOn(window, "setTimeout");
    const transport = new SocketIoTerminalTransport("http://terminal.test");
    readySession();
    const execution = transport.execute("vim edit.txt", vi.fn(), {
      interactive: true,
    });
    await Promise.resolve();

    expect(timeoutSpy).not.toHaveBeenCalled();
    transport.write("\x1b:wq\r");
    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:input", {
      data: "\x1b:wq\r",
      messageId: "message-1",
    });

    commandMessage("TEXT_MESSAGE_END", "", "/workspace");
    await execution;
  });

  it("buffers resize until the session is ready", () => {
    const transport = new SocketIoTerminalTransport("http://terminal.test");

    transport.resize({ cols: 100, rows: 40 });
    expect(socketHarness.socket.emit).not.toHaveBeenCalledWith(
      "terminal:resize",
      expect.anything(),
    );

    readySession();
    expect(socketHarness.socket.emit).toHaveBeenCalledWith("terminal:resize", {
      cols: 100,
      rows: 40,
    });
  });
});

function readySession(currentDirectory = "/workspace") {
  socketHarness.handlers.get("terminal:session")?.({
    currentDirectory,
    delta: "",
    messageId: "session-1",
    timestamp: 1,
    type: "TEXT_MESSAGE_CONTENT",
  });
}

function commandMessage(
  type: string,
  delta: string,
  currentDirectory?: string,
) {
  socketHarness.handlers.get("command:message")?.({
    ...(currentDirectory ? { currentDirectory } : {}),
    delta,
    messageId: "message-1",
    timestamp: 1,
    type,
  });
}
