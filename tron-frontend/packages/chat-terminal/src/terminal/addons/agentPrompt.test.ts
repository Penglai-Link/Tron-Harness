// @vitest-environment jsdom

import type { IDisposable, Terminal as XTerminal } from "@xterm/xterm";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AgentPromptAddon } from "./agentPrompt";

class TerminalStub {
  readonly cols = 80;
  readonly buffer = { active: { cursorX: 0 } };
  readonly element = document.createElement("div");
  readonly options: XTerminal["options"] = { cursorInactiveStyle: "outline" };
  readonly textarea = document.createElement("textarea");
  readonly blur = vi.fn(() => this.textarea.blur());
  readonly focus = vi.fn(() => this.textarea.focus());
  readonly write = vi.fn();
  private writeParsedHandler?: () => void;

  constructor() {
    this.element.appendChild(this.textarea);
    document.body.appendChild(this.element);
  }

  onCursorMove = listenerStub;
  onWriteParsed = (handler: () => void): IDisposable => {
    this.writeParsedHandler = handler;
    return { dispose: vi.fn() };
  };
  onResize = listenerStub;
  onData = listenerStub;

  emitWriteParsed() {
    this.writeParsedHandler?.();
  }

  registerMarker() {
    return { dispose: vi.fn() };
  }

  registerDecoration() {
    return undefined;
  }
}

function listenerStub(): IDisposable {
  return { dispose: vi.fn() };
}

describe("AgentPromptAddon cursor visibility", () => {
  beforeEach(() => {
    vi.stubGlobal("requestAnimationFrame", vi.fn(() => 1));
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.replaceChildren();
  });

  it("keeps the cursor hidden across terminal redraws and restores focus", () => {
    const terminal = new TerminalStub();
    const addon = new AgentPromptAddon({ text: "Ask AI" });
    addon.activate(terminal as unknown as XTerminal);
    terminal.textarea.focus();

    addon.setInputState(false, "", { hideCursor: true });

    expect(terminal.options.cursorInactiveStyle).toBe("none");
    expect(terminal.write).toHaveBeenCalledWith("\x1b[?25l");
    expect(terminal.blur).toHaveBeenCalledOnce();
    expect(
      terminal.element.classList.contains("ai-terminal-input-disabled"),
    ).toBe(true);

    terminal.emitWriteParsed();
    terminal.textarea.focus();

    expect(terminal.blur).toHaveBeenCalledTimes(2);
    expect(terminal.options.cursorInactiveStyle).toBe("none");

    addon.setInputState(false, "", { hideCursor: false });

    expect(terminal.options.cursorInactiveStyle).toBe("outline");
    expect(terminal.write).toHaveBeenLastCalledWith("\x1b[?25h");
    expect(terminal.focus).toHaveBeenCalledOnce();
    expect(
      terminal.element.classList.contains("ai-terminal-input-disabled"),
    ).toBe(false);
    addon.dispose();
  });

  it("does not focus a terminal that was inactive before the request", () => {
    const terminal = new TerminalStub();
    const addon = new AgentPromptAddon({ text: "Ask AI" });
    addon.activate(terminal as unknown as XTerminal);

    addon.setInputState(false, "", { hideCursor: true });
    addon.setInputState(false, "", { hideCursor: false });

    expect(terminal.focus).not.toHaveBeenCalled();
    addon.dispose();
  });
});
