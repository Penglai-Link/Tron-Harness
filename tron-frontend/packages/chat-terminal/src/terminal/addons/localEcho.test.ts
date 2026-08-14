// @vitest-environment jsdom
import type { IDisposable, Terminal as XTerminal } from "@xterm/xterm";
import { describe, expect, it, vi } from "vitest";

import { LocalEchoAddon } from "./localEcho";

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
  readonly cols: number;
  private readonly deferWrites: boolean;
  private pendingWriteCallbacks: Array<() => void> = [];
  private dataHandler?: (data: string) => void;

  constructor(cols = 80, cursorX = 0, deferWrites = false) {
    this.cols = cols;
    this.deferWrites = deferWrites;
    this.buffer.active.cursorX = cursorX;
  }

  emitData(data: string) {
    this.dataHandler?.(data);
  }

  onData(handler: (data: string) => void): IDisposable {
    this.dataHandler = handler;
    return { dispose: vi.fn() };
  }

  onResize(): IDisposable {
    return { dispose: vi.fn() };
  }

  onScroll(): IDisposable {
    return { dispose: vi.fn() };
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
      const controlSequence = remaining.match(/^\u001b\[(\d*)([ABCDX])/);
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
});

describe("LocalEchoAddon input cursor tracking", () => {
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
});

function flushTasks() {
  return new Promise<void>((resolve) => window.setTimeout(resolve, 0));
}
