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
});

function readySession(currentDirectory = "/workspace") {
  socketHarness.handlers.get("terminal:session")?.({
    code: "0",
    data: { currentDirectory },
    message: "",
  });
}
