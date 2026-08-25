// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

const harness = vi.hoisted(() => {
  const promptRendered = vi.fn();
  const resizeDisposable = { dispose: vi.fn() };
  const terminal = {
    onResize: vi.fn(() => resizeDisposable),
    write: vi.fn(),
  };
  let localEchoOptions: unknown;

  return { localEchoOptions, promptRendered, resizeDisposable, terminal };
});

vi.mock("./terminal", () => ({
  isInteractiveShellCommand: vi.fn(() => false),
  Terminal: class {
    constructor(_container: HTMLElement, options: { localEcho?: unknown }) {
      harness.localEchoOptions = options.localEcho;
    }

    terminal = harness.terminal;

    dispose = vi.fn();

    markPromptRendered = harness.promptRendered;
  },
}));

vi.mock("./terminal/i18n", () => ({
  createChatTerminalI18n: vi.fn(() => ({})),
}));

vi.mock("./terminal/utils/terminalCommandDetector", () => ({
  TerminalCommandDetector: {
    shouldTriggerAgentMode: vi.fn(() => false),
  },
}));

import { initTerminal } from "./index";

describe("initTerminal", () => {
  afterEach(() => {
    harness.promptRendered.mockClear();
    harness.resizeDisposable.dispose.mockClear();
    harness.terminal.onResize.mockClear();
    harness.terminal.write.mockClear();
    harness.localEchoOptions = undefined;
  });

  it("waits for the server session directory before rendering the first prompt", () => {
    let onSessionChange:
      | ((session: { currentDirectory: string }) => void)
      | undefined;
    const transport = {
      cancel: vi.fn(),
      connect: vi.fn(() => new Promise<never>(() => undefined)),
      dispose: vi.fn(),
      execute: vi.fn(),
      onSessionChange: vi.fn((listener) => {
        onSessionChange = listener;
        return { dispose: vi.fn() };
      }),
      resize: vi.fn(),
      write: vi.fn(),
    };

    const terminal = initTerminal({
      el: document.createElement("div"),
      transport: { terminal: transport },
    });

    expect(harness.terminal.write).not.toHaveBeenCalled();
    expect(harness.promptRendered).not.toHaveBeenCalled();

    onSessionChange?.({
      currentDirectory: "/workspace/project",
    });

    expect(harness.terminal.write).toHaveBeenCalledWith(
      "\r\x1b[2K/workspace/project ",
    );
    expect(harness.promptRendered).toHaveBeenCalledOnce();

    terminal.dispose();
  });

  it("closes the host tab for exit without executing a shell command", async () => {
    const onExitCommand = vi.fn();
    const transport = {
      cancel: vi.fn(),
      connect: vi.fn(() => new Promise<never>(() => undefined)),
      dispose: vi.fn(),
      execute: vi.fn(),
      onSessionChange: vi.fn(() => ({ dispose: vi.fn() })),
      resize: vi.fn(),
      write: vi.fn(),
    };

    const terminal = initTerminal({
      el: document.createElement("div"),
      onExitCommand,
      transport: { terminal: transport },
    });
    const localEchoOptions = harness.localEchoOptions as {
      onBeforeCommand: (command: string) => Promise<boolean>;
    };

    await expect(localEchoOptions.onBeforeCommand("  exit  ")).resolves.toBe(true);
    expect(onExitCommand).toHaveBeenCalledOnce();
    expect(transport.execute).not.toHaveBeenCalled();

    terminal.dispose();
  });
});
