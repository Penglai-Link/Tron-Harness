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
  private dataHandler?: (data: string) => void;

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
    callback?.();
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
});

function flushTasks() {
  return new Promise<void>((resolve) => window.setTimeout(resolve, 0));
}
