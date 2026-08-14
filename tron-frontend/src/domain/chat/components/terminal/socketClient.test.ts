// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

const socketHarness = vi.hoisted(() => {
  const handlers = new Map<string, (...args: unknown[]) => void>();
  const socket = {
    connected: true,
    connect: vi.fn(),
    disconnect: vi.fn(),
    emit: vi.fn(),
    off: vi.fn(),
    on: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
      handlers.set(event, handler);
    }),
    once: vi.fn(),
  };

  return { handlers, socket };
});

vi.mock("socket.io-client", () => ({
  io: vi.fn(() => socketHarness.socket),
}));

import { TerminalSocketClient } from "./socketClient";

describe("TerminalSocketClient cancellation", () => {
  beforeEach(() => {
    socketHarness.handlers.clear();
    socketHarness.socket.emit.mockClear();
    socketHarness.socket.on.mockClear();
    vi.stubGlobal("crypto", {
      randomUUID: vi
        .fn()
        .mockReturnValueOnce("message-1")
        .mockReturnValueOnce("message-2"),
    });
  });

  it("cancels the active client message and ignores late output", async () => {
    const client = new TerminalSocketClient("http://terminal.test");
    readySession();
    const onOutput = vi.fn();
    const execution = client.execute("ping", onOutput);
    await Promise.resolve();

    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:execute", {
      command: "ping",
      messageId: "message-1",
    });

    client.cancelCurrent();
    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:cancel", {
      messageId: "message-1",
    });

    socketHarness.handlers.get("command:message")?.({
      delta: "late output",
      messageId: "message-1",
      timestamp: 1,
      type: "TEXT_MESSAGE_CONTENT",
    });
    expect(onOutput).not.toHaveBeenCalled();

    socketHarness.handlers.get("command:message")?.({
      currentDirectory: "/workspace",
      delta: "",
      messageId: "message-1",
      timestamp: 2,
      type: "TEXT_MESSAGE_END",
    });
    await expect(execution).resolves.toEqual({
      currentDirectory: "/workspace",
    });
  });

  it("uses a new message id after cancellation", async () => {
    const client = new TerminalSocketClient("http://terminal.test");
    readySession();
    const firstExecution = client.execute("ping", vi.fn());
    await Promise.resolve();
    client.cancelCurrent();
    socketHarness.handlers.get("command:message")?.({
      currentDirectory: "/workspace",
      delta: "",
      messageId: "message-1",
      timestamp: 1,
      type: "TEXT_MESSAGE_END",
    });
    await firstExecution;

    void client.execute("pwd", vi.fn());
    await Promise.resolve();

    expect(socketHarness.socket.emit).toHaveBeenLastCalledWith(
      "command:execute",
      { command: "pwd", messageId: "message-2" },
    );
    client.cancelCurrent();
  });

  it("publishes the initial directory when a terminal session is ready", () => {
    const onSessionChange = vi.fn();
    new TerminalSocketClient("http://terminal.test", { onSessionChange });

    readySession("/workspace/project");

    expect(onSessionChange).toHaveBeenCalledWith("/workspace/project");
  });

  it("rejects connection when the PTY session reports an envelope error", async () => {
    const client = new TerminalSocketClient("http://terminal.test");
    const connection = client.connect();

    socketHarness.handlers.get("terminal:session")?.({
      code: "TERMINAL_SESSION_ERROR",
      data: {},
      message: "终端会话无法启动",
    });

    await expect(connection).rejects.toThrow("终端会话无法启动");
  });

  it("waits for the PTY session before sending the first command", async () => {
    const client = new TerminalSocketClient("http://terminal.test");
    const execution = client.execute("pwd", vi.fn());
    await Promise.resolve();

    expect(socketHarness.socket.emit).not.toHaveBeenCalledWith(
      "command:execute",
      expect.anything(),
    );

    readySession();
    await Promise.resolve();

    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:execute", {
      command: "pwd",
      messageId: "message-1",
    });
    client.cancelCurrent();
    socketHarness.handlers.get("command:message")?.({
      currentDirectory: "/workspace",
      delta: "",
      messageId: "message-1",
      timestamp: 1,
      type: "TEXT_MESSAGE_END",
    });
    await execution;
  });

  it("forwards interactive input without a command timeout", async () => {
    const timeoutSpy = vi.spyOn(window, "setTimeout");
    const client = new TerminalSocketClient("http://terminal.test");
    readySession();

    const execution = client.execute("vim edit.txt", vi.fn(), {
      interactive: true,
    });
    await Promise.resolve();

    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:execute", {
      command: "vim edit.txt",
      interactive: true,
      messageId: "message-1",
    });
    expect(timeoutSpy).not.toHaveBeenCalled();

    client.sendInput("i");
    expect(socketHarness.socket.emit).toHaveBeenCalledWith("command:input", {
      data: "i",
      messageId: "message-1",
    });

    socketHarness.handlers.get("command:message")?.({
      currentDirectory: "/workspace",
      delta: "",
      messageId: "message-1",
      timestamp: 1,
      type: "TEXT_MESSAGE_END",
    });
    await execution;
  });

  it("buffers the latest terminal size until the PTY session is ready", () => {
    const client = new TerminalSocketClient("http://terminal.test");

    client.resize({ cols: 100, rows: 40 });
    expect(socketHarness.socket.emit).not.toHaveBeenCalledWith(
      "terminal:resize",
      expect.anything(),
    );

    readySession();
    expect(socketHarness.socket.emit).toHaveBeenCalledWith("terminal:resize", {
      cols: 100,
      rows: 40,
    });

    client.resize({ cols: 120, rows: 50 });
    expect(socketHarness.socket.emit).toHaveBeenLastCalledWith(
      "terminal:resize",
      { cols: 120, rows: 50 },
    );
  });
});

function readySession(currentDirectory = "/workspace") {
  socketHarness.handlers.get("terminal:session")?.({
    code: "0",
    data: { currentDirectory },
    message: "",
  });
}
