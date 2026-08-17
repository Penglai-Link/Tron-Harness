import {
  Terminal as XTerminal,
  type IDecoration,
  type IDisposable,
  type IMarker,
  type ITerminalAddon,
} from "@xterm/xterm";
import type { AgentPromptAddonOptions } from "../types/agentPrompt";

const PROMPT_RESIZE_DEBOUNCE_MS = 100;

export class AgentPromptAddon implements ITerminalAddon {
  private disposables: IDisposable[] = [];
  private decoration?: IDecoration;
  private marker?: IMarker;
  private terminal?: XTerminal;
  private positionAnimationFrame?: number;
  private resizeDebounceTimer?: number;
  private resizeSuppressed = false;
  private inputEnabled = true;
  private hasInput = false;
  private isInputStateManaged = false;
  private standaloneInput = "";
  private visible = true;

  constructor(private readonly options: AgentPromptAddonOptions) {}

  activate(terminal: XTerminal) {
    if (!terminal.element) {
      throw new Error("AgentPromptAddon must be loaded after terminal.open().");
    }

    this.terminal = terminal;
    this.bindTracking(terminal);
    this.syncVisibility();
    this.updateDecoration();
  }

  dispose() {
    for (const disposable of this.disposables) {
      disposable.dispose();
    }

    this.disposables = [];
    if (this.positionAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.positionAnimationFrame);
      this.positionAnimationFrame = undefined;
    }
    if (this.resizeDebounceTimer !== undefined) {
      window.clearTimeout(this.resizeDebounceTimer);
      this.resizeDebounceTimer = undefined;
    }
    this.resizeSuppressed = false;
    this.disposeDecoration();
    this.terminal?.element?.classList.remove("ai-terminal-input-disabled");
    this.terminal = undefined;
  }

  setVisible(visible: boolean) {
    this.visible = visible;
    this.schedulePositionUpdate();
  }

  refreshLocale() {
    this.mountPrompt(this.decoration?.element);
  }

  setInputState(
    enabled: boolean,
    input: string,
    options: { hideCursor?: boolean } = {},
  ) {
    this.isInputStateManaged = true;
    this.inputEnabled = enabled;
    this.hasInput = input.length > 0;
    this.terminal?.element?.classList.toggle(
      "ai-terminal-input-disabled",
      options.hideCursor ?? !enabled,
    );
    this.syncVisibility();
  }

  private getPromptText() {
    return (
      this.options.text ??
      this.options.i18n?.t(this.options.textKey ?? "agentTips") ??
      ""
    );
  }

  private bindTracking(terminal: XTerminal) {
    this.disposables.push(
      terminal.onCursorMove(() => {
        this.schedulePositionUpdate();
      }),
      terminal.onWriteParsed(() => {
        this.schedulePositionUpdate();
      }),
      terminal.onResize(() => {
        this.suppressDuringResize();
      }),
      terminal.onData((data) => {
        this.handleStandaloneInput(data);
      }),
    );
  }

  private handleStandaloneInput(data: string) {
    if (this.isInputStateManaged || !this.inputEnabled) {
      return;
    }

    if (data === "\r") {
      this.standaloneInput = "";
    } else if (data === "\u007f") {
      this.standaloneInput = Array.from(this.standaloneInput)
        .slice(0, -1)
        .join("");
    } else if (!data.startsWith("\x1b")) {
      this.standaloneInput += data;
    }

    this.hasInput = this.standaloneInput.length > 0;
    this.syncVisibility();
  }

  private syncVisibility() {
    this.setVisible(this.inputEnabled && !this.hasInput);
    this.schedulePositionUpdate();
  }

  private schedulePositionUpdate() {
    if (this.positionAnimationFrame !== undefined) {
      return;
    }

    this.positionAnimationFrame = window.requestAnimationFrame(() => {
      this.positionAnimationFrame = undefined;
      this.updateDecoration();
    });
  }

  private updateDecoration() {
    if (!this.terminal || !this.visible || this.resizeSuppressed) {
      this.disposeDecoration();
      return;
    }

    const buffer = this.terminal.buffer.active;
    const gapCells = this.options.gapCells ?? 1;
    const absoluteColumn = buffer.cursorX + gapCells;
    const rowOffset = Math.floor(absoluteColumn / this.terminal.cols);
    const column = absoluteColumn % this.terminal.cols;
    this.disposeDecoration();
    const marker = this.terminal.registerMarker(rowOffset);
    const decoration = this.terminal.registerDecoration({
      marker,
      anchor: "left",
      x: column,
      width: Math.max(1, this.terminal.cols - column),
      height: 1,
      layer: "top",
    });

    if (!decoration) {
      marker.dispose();
      return;
    }

    this.marker = marker;
    this.decoration = decoration;
    decoration.onRender((element) => {
      if (this.decoration === decoration) {
        this.mountPrompt(element);
      }
    });
    if (decoration.element) {
      this.mountPrompt(decoration.element);
    }
  }

  private mountPrompt(element?: HTMLElement) {
    if (!element || !this.terminal) {
      return;
    }

    element.classList.add("ai-terminal-agent-prompt");
    element.textContent = this.getPromptText();
    element.style.overflow = "visible";
    element.style.setProperty(
      "--ai-terminal-agent-prompt-color",
      this.options.color ?? "rgba(255, 255, 255, 0.48)",
    );
    element.style.setProperty(
      "--ai-terminal-agent-prompt-font-size",
      `${this.terminal.options.fontSize ?? 14}px`,
    );
  }

  private disposeDecoration() {
    const decoration = this.decoration;
    const marker = this.marker;

    this.decoration = undefined;
    this.marker = undefined;
    decoration?.dispose();
    marker?.dispose();
  }

  private suppressDuringResize() {
    this.resizeSuppressed = true;
    this.disposeDecoration();

    if (this.resizeDebounceTimer !== undefined) {
      window.clearTimeout(this.resizeDebounceTimer);
    }

    this.resizeDebounceTimer = window.setTimeout(() => {
      this.resizeDebounceTimer = undefined;
      this.resizeSuppressed = false;
      this.schedulePositionUpdate();
    }, PROMPT_RESIZE_DEBOUNCE_MS);
  }
}
