// @vitest-environment jsdom
import type { IDisposable, Terminal as XTerminal } from "@xterm/xterm";
import { describe, expect, it, vi } from "vitest";

import { LocalEchoAddon } from "./localEcho";
import type { ExplainDialogPosition } from "../types/ui";

class TerminalStub {
  readonly buffer = {
    active: {
      baseY: 0,
      cursorX: 0,
      cursorY: 0,
      getLine: () => undefined,
    },
  };
  readonly element = document.createElement("div");
  readonly writes: string[] = [];
  clearCount = 0;
  cols: number;
  private readonly deferWrites: boolean;
  private pendingWriteCallbacks: Array<() => void> = [];
  private dataHandler?: (data: string) => void;
  private keyEventHandler?: (event: KeyboardEvent) => boolean;
  private resizeHandler?: () => void;

  constructor(cols = 80, cursorX = 0, deferWrites = false) {
    this.cols = cols;
    this.deferWrites = deferWrites;
    this.buffer.active.cursorX = cursorX;
  }

  emitData(data: string) {
    this.dataHandler?.(data);
  }

  emitResize(cols: number) {
    this.cols = cols;
    this.resizeHandler?.();
  }

  emitKeyEvent(event: KeyboardEvent) {
    return this.keyEventHandler?.(event);
  }

  onData(handler: (data: string) => void): IDisposable {
    this.dataHandler = handler;
    return { dispose: vi.fn() };
  }

  onResize(handler: () => void): IDisposable {
    this.resizeHandler = handler;
    return { dispose: vi.fn() };
  }

  attachCustomKeyEventHandler(handler: (event: KeyboardEvent) => boolean) {
    this.keyEventHandler = handler;
  }

  onScroll(): IDisposable {
    return { dispose: vi.fn() };
  }

  clear() {
    this.clearCount += 1;
    this.buffer.active.baseY = 0;
    this.buffer.active.cursorY = 0;
  }

  write(data: string, callback?: () => void) {
    this.writes.push(data);
    this.applyWrite(data);
    if (callback && this.deferWrites) {
      this.pendingWriteCallbacks.push(callback);
    } else {
      callback?.();
    }
  }

  flushWrites() {
    const callbacks = this.pendingWriteCallbacks.splice(0);
    callbacks.forEach((callback) => callback());
  }

  private applyWrite(data: string) {
    let remaining = data;

    while (remaining) {
      const controlSequence = remaining.match(/^\u001b\[(\d*)([ABCDKX])/);
      if (controlSequence) {
        const amount = Number(controlSequence[1] || 1);
        const command = controlSequence[2];

        if (command === "A") {
          this.buffer.active.cursorY -= amount;
        } else if (command === "B") {
          this.buffer.active.cursorY += amount;
        } else if (command === "C") {
          this.buffer.active.cursorX += amount;
        } else if (command === "D") {
          this.buffer.active.cursorX = Math.max(
            0,
            this.buffer.active.cursorX - amount,
          );
        }

        remaining = remaining.slice(controlSequence[0].length);
        continue;
      }

      const character = remaining[0];
      remaining = remaining.slice(1);

      if (character === "\r") {
        this.buffer.active.cursorX = 0;
        continue;
      }

      if (character === "\t") {
        if (this.buffer.active.cursorX < this.cols) {
          this.buffer.active.cursorX = Math.min(
            this.cols - 1,
            this.buffer.active.cursorX +
              (8 - (this.buffer.active.cursorX % 8)),
          );
        }
        continue;
      }

      if (character < " ") {
        continue;
      }

      if (this.buffer.active.cursorX >= this.cols) {
        this.buffer.active.cursorY += 1;
        this.buffer.active.cursorX = 0;
      }

      this.buffer.active.cursorX += 1;
    }
  }
}

describe("LocalEchoAddon command cancellation", () => {
  it("handles Ctrl+C once while executing and restores input", async () => {
    const terminal = new TerminalStub();
    const inputStates: boolean[] = [];
    let finishCommand: (() => void) | undefined;
    const onCancelCommand = vi.fn(() => finishCommand?.());
    const addon = new LocalEchoAddon(
      {
        enabled: true,
        onCancelCommand,
        onCommand: () =>
          new Promise<void>((resolve) => {
            finishCommand = resolve;
          }),
        prompt: "$ ",
      },
      (enabled) => inputStates.push(enabled),
    );
    addon.activate(terminal as unknown as XTerminal);

    terminal.emitData("ping");
    await flushTasks();
    terminal.emitData("\r");
    await flushTasks();
    terminal.emitData("\u0003");
    terminal.emitData("\u0003");
    await flushTasks();

    expect(onCancelCommand).toHaveBeenCalledTimes(1);
    expect(terminal.writes.filter((write) => write === "^C\r\n")).toHaveLength(1);
    expect(terminal.writes.at(-1)).toBe("$ \x1b[?25h");
    expect(inputStates.at(-1)).toBe(true);
  });

  it("forwards interactive input and preserves raw PTY output", async () => {
    const terminal = new TerminalStub();
    const onCommandInput = vi.fn();
    let onOutput: ((output: string) => void) | undefined;
    let finishCommand: (() => void) | undefined;
    const addon = new LocalEchoAddon({
      enabled: true,
      isInteractiveCommand: (command) => command === "vim edit.txt",
      onCommand: (_command, nextOutput, context) => {
        expect(context).toEqual({ interactive: true });
        onOutput = nextOutput;
        return new Promise<void>((resolve) => {
          finishCommand = resolve;
        });
      },
      onCommandInput,
      prompt: "$ ",
    });
    addon.activate(terminal as unknown as XTerminal);

    terminal.emitData("vim edit.txt");
    terminal.emitData("\r");
    await flushTasks();

    onOutput?.("\x1b[?1049hfirst\nsecond");
    terminal.emitData("i");
    terminal.emitData("hello");
    terminal.emitData("\x1b");
    terminal.emitData(":wq\r");
    await flushTasks();

    expect(onCommandInput.mock.calls.map(([data]) => data)).toEqual([
      "i",
      "hello",
      "\x1b",
      ":wq\r",
    ]);
    expect(terminal.writes).toContain("\x1b[?1049hfirst\nsecond");
    expect(terminal.writes).not.toContain("\x1b[?1049hfirst\r\nsecond");

    finishCommand?.();
    await flushTasks();

    expect(terminal.writes.at(-1)).toBe("$ \x1b[?25h");
  });

  it("keeps a cursor and forwards input for prompt-driven shell commands", async () => {
    const terminal = new TerminalStub();
    const onCommandInput = vi.fn();
    let finishCommand: (() => void) | undefined;
    const addon = new LocalEchoAddon({
      enabled: true,
      onCommand: (_command, onOutput, context) => {
        expect(context).toEqual({ interactive: true });
        onOutput("package name: ");
        return new Promise<void>((resolve) => {
          finishCommand = resolve;
        });
      },
      onCommandInput,
      prompt: "$ ",
    });
    addon.activate(terminal as unknown as XTerminal);

    terminal.emitData("npm init");
    terminal.emitData("\r");
    await flushTasks();

    expect(terminal.writes).toContain("\r\n\x1b[?25h");
    expect(terminal.writes).not.toContain("\r\n\x1b[?25l");
    expect(terminal.writes).toContain("package name: ");

    terminal.emitData("demo\r");
    terminal.emitData("\u000b");
    await flushTasks();

    expect(onCommandInput.mock.calls.map(([data]) => data)).toEqual([
      "demo\r",
      "\u000b",
    ]);

    finishCommand?.();
    await flushTasks();
  });

  it("allows callers to disable command input", async () => {
    const terminal = new TerminalStub();
    const onCommandInput = vi.fn();
    const addon = new LocalEchoAddon({
      enabled: true,
      isCommandInputEnabled: () => false,
      onCommand: (_command, _onOutput, context) => {
        expect(context).toEqual({ interactive: false });
      },
      onCommandInput,
      prompt: "$ ",
    });
    addon.activate(terminal as unknown as XTerminal);

    terminal.emitData("npm install vue");
    terminal.emitData("\r");
    await flushTasks();
    terminal.emitData("y");
    await flushTasks();

    expect(terminal.writes).toContain("\r\n\x1b[?25l");
    expect(onCommandInput).not.toHaveBeenCalled();
  });
});

describe("LocalEchoAddon input cursor tracking", () => {
  it("clears the terminal with Command+K while preserving the current input", async () => {
    const terminal = new TerminalStub();
    const inputStates: string[] = [];
    const addon = new LocalEchoAddon(
      { enabled: true, prompt: "$ " },
      (_enabled, input) => inputStates.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);
    terminal.write("$ ");
    addon.markPromptRendered();
    terminal.emitData("git status");
    await flushTasks();

    const commandK = new KeyboardEvent("keydown", {
      cancelable: true,
      key: "k",
      metaKey: true,
    });
    expect(terminal.emitKeyEvent(commandK)).toBe(false);
    await flushTasks();

    expect(terminal.clearCount).toBe(1);
    expect(commandK.defaultPrevented).toBe(true);
    expect(terminal.writes.at(-1)).toBe("\r\x1b[2K$ git status");
    expect(inputStates.at(-1)).toBe("git status");

    terminal.emitData("\u007f");
    await flushTasks();
    expect(inputStates.at(-1)).toBe("git statu");
  });

  it("does not clear the terminal with Control+K", async () => {
    const terminal = new TerminalStub();
    const addon = new LocalEchoAddon({ enabled: true, prompt: "$ " });
    addon.activate(terminal as unknown as XTerminal);

    const controlK = new KeyboardEvent("keydown", {
      cancelable: true,
      ctrlKey: true,
      key: "k",
    });

    expect(terminal.emitKeyEvent(controlK)).toBe(true);
    expect(controlK.defaultPrevented).toBe(false);
    expect(terminal.clearCount).toBe(0);
  });

  it("navigates executed shell commands with the arrow keys", async () => {
    const terminal = new TerminalStub();
    const inputStates: string[] = [];
    const submittedCommands: string[] = [];
    const addon = new LocalEchoAddon(
      {
        enabled: true,
        onCommand: (command) => {
          submittedCommands.push(command);
        },
        prompt: "$ ",
      },
      (_enabled, input) => inputStates.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);
    terminal.write("$ ");
    addon.markPromptRendered();

    terminal.emitData("pwd");
    terminal.emitData("\r");
    await flushTasks();
    terminal.emitData("ls -la");
    terminal.emitData("\r");
    await flushTasks();

    terminal.emitData("draft");
    await flushTasks();
    terminal.emitData("\x1b[A");
    await flushTasks();
    expect(inputStates.at(-1)).toBe("ls -la");

    terminal.emitData("\x1b[A");
    await flushTasks();
    expect(inputStates.at(-1)).toBe("pwd");

    terminal.emitData("\x1b[B");
    await flushTasks();
    expect(inputStates.at(-1)).toBe("ls -la");

    terminal.emitData("\x1b[B");
    await flushTasks();
    expect(inputStates.at(-1)).toBe("draft");
    expect(submittedCommands).toEqual(["pwd", "ls -la"]);
  });

  it("does not add agent-handled input to shell command history", async () => {
    const terminal = new TerminalStub();
    const inputStates: string[] = [];
    const onCommand = vi.fn();
    const addon = new LocalEchoAddon(
      {
        enabled: true,
        onBeforeCommand: () => true,
        onCommand,
        prompt: "$ ",
      },
      (_enabled, input) => inputStates.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);
    terminal.write("$ ");
    addon.markPromptRendered();

    terminal.emitData("summarize the logs");
    terminal.emitData("\r");
    await flushTasks();
    terminal.emitData("\x1b[A");
    await flushTasks();

    expect(onCommand).not.toHaveBeenCalled();
    expect(inputStates.at(-1)).toBe("");
  });

  it("writes pasted text in one terminal operation", async () => {
    const terminal = new TerminalStub();
    const addon = new LocalEchoAddon({ enabled: true });
    addon.activate(terminal as unknown as XTerminal);

    const pastedText = "a long pasted command with many characters";
    terminal.emitData(pastedText);
    await flushTasks();

    expect(terminal.writes).toEqual([pastedText]);
  });

  it("hides the agent prompt before a long write is parsed", async () => {
    const terminal = new TerminalStub(80, 0, true);
    const inputs: string[] = [];
    const addon = new LocalEchoAddon(
      { enabled: true },
      (_enabled, input) => inputs.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);

    const pastedText = "a long pasted command";
    terminal.emitData(pastedText);
    await Promise.resolve();

    expect(inputs.at(-1)).toBe(pastedText);
    terminal.flushWrites();
    await flushTasks();
  });

  it("deletes tabs according to their rendered cell width", async () => {
    const terminal = new TerminalStub(10);
    const inputs: string[] = [];
    const addon = new LocalEchoAddon(
      { enabled: true },
      (_enabled, input) => inputs.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);

    terminal.emitData("a");
    terminal.emitData("\t");
    terminal.emitData("\t");
    await flushTasks();

    terminal.emitData("\u007f");
    await flushTasks();
    expect(inputs.at(-1)).toBe("a\t");
    expect(terminal.buffer.active.cursorX).toBe(8);

    terminal.emitData("\u007f");
    await flushTasks();
    expect(inputs.at(-1)).toBe("a");
    expect(terminal.buffer.active.cursorX).toBe(1);

    terminal.emitData("\u007f");
    await flushTasks();
    expect(inputs.at(-1)).toBe("");
    expect(terminal.buffer.active.cursorX).toBe(0);
  });

  it("deletes wrapped input back to its original cursor position", async () => {
    const terminal = new TerminalStub(10, 8);
    const inputs: string[] = [];
    const addon = new LocalEchoAddon(
      { enabled: true },
      (_enabled, input) => inputs.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);

    terminal.emitData("abc");
    await flushTasks();
    expect(terminal.buffer.active.cursorY).toBe(1);
    expect(terminal.buffer.active.cursorX).toBe(1);

    terminal.emitData("\u007f");
    terminal.emitData("\u007f");
    terminal.emitData("\u007f");
    await flushTasks();

    expect(inputs.at(-1)).toBe("");
    expect(terminal.buffer.active.cursorY).toBe(0);
    expect(terminal.buffer.active.cursorX).toBe(8);
  });

  it("redraws the complete prompt and input after shrinking and expanding", async () => {
    const terminal = new TerminalStub(20);
    const inputs: string[] = [];
    const addon = new LocalEchoAddon(
      { enabled: true, prompt: "/workspace " },
      (_enabled, input) => inputs.push(input),
    );
    addon.activate(terminal as unknown as XTerminal);

    terminal.write("/workspace ");
    addon.markPromptRendered();
    terminal.emitData("echo");
    await flushTasks();

    terminal.emitResize(8);
    await flushResize();

    expect(terminal.writes.at(-1)).toBe("\r\x1b[2K/workspace echo");
    expect(terminal.buffer.active.cursorY).toBe(1);

    terminal.emitResize(20);
    await flushResize();

    expect(terminal.writes.at(-1)).toBe(
      "\r\x1b[2K\x1b[1A\r\x1b[2K/workspace echo",
    );
    expect(terminal.buffer.active.cursorY).toBe(0);
    expect(terminal.buffer.active.cursorX).toBe(15);

    terminal.emitData("\u007f");
    await flushTasks();

    expect(inputs.at(-1)).toBe("ech");
    expect(terminal.buffer.active.cursorX).toBe(14);
  });
});

describe("LocalEchoAddon explanation streaming", () => {
  it("renders received chunks before the explanation request completes", async () => {
    let finishRequest: (() => void) | undefined;
    const addon = new LocalEchoAddon({
      onExplain: async (_payload, { onChunk }) => {
        onChunk?.("first");
        await new Promise<void>((resolve) => {
          finishRequest = resolve;
        });
        onChunk?.(" second");
        return "first second";
      },
    });
    const { dialog, request } = startExplanation(addon, "printf first");

    await Promise.resolve();

    expect(dialog.textContent).toContain("first");
    expect(dialog.querySelector(".ai-terminal-explain-dialog--streaming")).not.toBeNull();

    finishRequest?.();
    await request;

    expect(dialog.textContent).toContain("first second");
    expect(dialog.querySelector(".ai-terminal-explain-dialog--success")).not.toBeNull();
  });

  it("ignores chunks from an explanation superseded by refresh", async () => {
    let firstSignal: AbortSignal | undefined;
    let emitFirstChunk: ((delta: string) => void) | undefined;
    let finishFirstRequest: ((content: string) => void) | undefined;
    const addon = new LocalEchoAddon({
      onExplain: (payload, context) => {
        if (payload.command === "first") {
          firstSignal = context.signal;
          emitFirstChunk = context.onChunk;
          return new Promise<string>((resolve) => {
            finishFirstRequest = resolve;
          });
        }

        context.onChunk?.("new content");
        return Promise.resolve("new content");
      },
    });
    const first = startExplanation(addon, "first");
    const second = startExplanation(addon, "second", first.dialog);

    emitFirstChunk?.("stale content");
    finishFirstRequest?.("stale content");
    await Promise.all([first.request, second.request]);

    expect(firstSignal?.aborted).toBe(true);
    expect(first.dialog.textContent).toContain("new content");
    expect(first.dialog.textContent).not.toContain("stale content");
  });

  it("keeps partial streamed content when the explanation fails", async () => {
    const addon = new LocalEchoAddon({
      onExplain: async (_payload, { onChunk }) => {
        onChunk?.("partial content");
        throw new Error("model failed");
      },
    });
    const { dialog, request } = startExplanation(addon, "failing command");

    await request;

    expect(dialog.textContent).toContain("partial content");
    expect(dialog.textContent).toContain("model failed");
    expect(dialog.querySelector(".ai-terminal-explain-dialog--error")).not.toBeNull();
  });
});

function startExplanation(
  addon: LocalEchoAddon,
  command: string,
  existingDialog?: HTMLElement,
) {
  const dialog = existingDialog ?? document.createElement("div");
  if (!dialog.isConnected) {
    document.body.appendChild(dialog);
  }
  const internals = addon as unknown as {
    currentDialog?: HTMLElement;
    renderExplanation: (
      dialog: HTMLElement,
      command: string,
      output: string,
      position: ExplainDialogPosition,
    ) => Promise<void>;
  };
  internals.currentDialog = dialog;

  return {
    dialog,
    request: internals.renderExplanation(dialog, command, "output", {
      left: 0,
      top: 0,
    }),
  };
}

function flushTasks() {
  return new Promise<void>((resolve) => window.setTimeout(resolve, 0));
}

function flushResize() {
  return new Promise<void>((resolve) => window.setTimeout(resolve, 120));
}
