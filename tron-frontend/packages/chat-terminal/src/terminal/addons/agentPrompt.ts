import {
  Terminal as XTerminal,
  type IDisposable,
  type ITerminalAddon,
} from "@xterm/xterm";
import type { AgentPromptAddonOptions } from "../types/agentPrompt";
import { getTerminalCellMetrics } from "../utils";

export class AgentPromptAddon implements ITerminalAddon {
  private disposables: IDisposable[] = [];
  private promptElement?: HTMLDivElement;
  private terminal?: XTerminal;
  private viewportElement?: HTMLElement;
  private positionAnimationFrame?: number;
  private inputEnabled = true;
  private hasInput = false;
  private isInputStateManaged = false;
  private standaloneInput = "";
  private readonly handleViewportScroll = () => {
    this.schedulePositionUpdate();
  };

  constructor(private readonly options: AgentPromptAddonOptions) {}

  activate(terminal: XTerminal) {
    if (!terminal.element) {
      throw new Error("AgentPromptAddon must be loaded after terminal.open().");
    }

    this.terminal = terminal;
    this.promptElement = this.createPromptElement(terminal);
    terminal.element.appendChild(this.promptElement);

    this.bindTracking(terminal);
    this.syncVisibility();
    this.updatePosition();
  }

  dispose() {
    for (const disposable of this.disposables) {
      disposable.dispose();
    }

    this.disposables = [];
    this.viewportElement?.removeEventListener(
      "scroll",
      this.handleViewportScroll,
    );
    this.viewportElement = undefined;
    if (this.positionAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.positionAnimationFrame);
      this.positionAnimationFrame = undefined;
    }
    this.promptElement?.remove();
    this.promptElement = undefined;
    this.terminal?.element?.classList.remove("ai-terminal-input-disabled");
    this.terminal = undefined;
  }

  setVisible(visible: boolean) {
    if (!this.promptElement) {
      return;
    }

    this.promptElement.style.display = visible ? "block" : "none";
  }

  refreshLocale() {
    if (!this.promptElement) {
      return;
    }

    this.promptElement.textContent = this.getPromptText();
    this.schedulePositionUpdate();
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

  private createPromptElement(terminal: XTerminal) {
    const promptElement = document.createElement("div");

    promptElement.textContent = this.getPromptText();
    promptElement.className = "ai-terminal-agent-prompt";
    promptElement.style.setProperty(
      "--ai-terminal-agent-prompt-color",
      this.options.color ?? "rgba(255, 255, 255, 0.48)",
    );
    promptElement.style.setProperty(
      "--ai-terminal-agent-prompt-font-size",
      `${terminal.options.fontSize ?? 14}px`,
    );

    return promptElement;
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
        this.schedulePositionUpdate();
      }),
      terminal.onScroll(() => {
        this.schedulePositionUpdate();
      }),
      terminal.onData((data) => {
        this.handleStandaloneInput(data);
      }),
    );

    const viewport =
      terminal.element?.querySelector<HTMLElement>(".xterm-viewport");

    if (viewport) {
      viewport.addEventListener("scroll", this.handleViewportScroll, {
        passive: true,
      });
      this.viewportElement = viewport;
    }
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
      this.updatePosition();
    });
  }

  private updatePosition() {
    if (!this.promptElement || !this.terminal) {
      return;
    }

    const cell = getTerminalCellMetrics(this.terminal);
    const buffer = this.terminal.buffer.active;
    const gapCells = this.options.gapCells ?? 1;
    const viewportRow = buffer.baseY + buffer.cursorY - buffer.viewportY;

    this.promptElement.style.left = `${
      (buffer.cursorX + gapCells) * cell.width
    }px`;
    this.promptElement.style.top = `${viewportRow * cell.height}px`;
    this.promptElement.style.setProperty(
      "--ai-terminal-agent-prompt-line-height",
      `${cell.height}px`,
    );
  }
}
