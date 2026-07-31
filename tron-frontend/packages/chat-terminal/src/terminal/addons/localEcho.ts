import {
  Terminal as XTerminal,
  type IDisposable,
  type ITerminalAddon,
} from "@xterm/xterm";
import type { LocalEchoAddonOptions } from "../types/localEcho";
import { createExplainButton } from "../ui/explainButton";
import { createExplainDialog, renderExplainDialog } from "../ui/explainDialog";
import {
  getTerminalCellMetrics,
  hasVisibleTerminalOutput,
  isControlCharacter,
  normalizeTerminalOutput,
  writeTerminalAsync,
} from "../utils";
import type { ExplainDialogPosition } from "../types/ui";

interface ExplainButtonAnchor {
  column: number;
  row: number;
}

interface ExplainButtonViewportAnchor {
  left: number;
  row: number;
  top: number;
  size: number;
}

interface ExplainButtonInstance {
  anchor: ExplainButtonAnchor;
  element: HTMLElement;
  isVisible: boolean;
  left: number;
  size: number;
  top: number;
}

const EXPLAIN_DIALOG_WIDTH = 400;
const EXPLAIN_DIALOG_MAX_HEIGHT = 180;
const EXPLAIN_DIALOG_MARGIN = 12;
const EXPLAIN_DIALOG_ANCHOR_OFFSET_Y = 8;
const EXPLAIN_RENDER_INTERVAL_MS = 50;

export class LocalEchoAddon implements ITerminalAddon {
  private disposable?: IDisposable;
  private scrollDisposable?: IDisposable;
  private resizeDisposable?: IDisposable;
  private viewportElement?: HTMLElement;
  private scrollAnimationFrame?: number;
  private explanationRequestVersion = 0;
  private explanationAbortController?: AbortController;
  private explanationRenderTimer?: number;
  private readonly handleViewportScroll = () => {
    this.scheduleExplainButtonPositionUpdate();
  };
  private currentInput = "";
  private isExecuting = false;
  private terminal?: XTerminal;
  private explainElements: HTMLElement[] = [];
  private explainButtons: ExplainButtonInstance[] = [];
  private currentDialog?: HTMLElement;
  private readonly handleDocumentPointerDown = (event: PointerEvent) => {
    const target = event.target;
    if (
      this.currentDialog &&
      target instanceof Node &&
      !this.currentDialog.contains(target)
    ) {
      this.closeExplainDialog();
    }
  };

  constructor(
    private readonly options: LocalEchoAddonOptions = {},
    private readonly onInputStateChange?: (
      enabled: boolean,
      input: string,
    ) => void,
  ) {}

  activate(terminal: XTerminal) {
    if (this.options.enabled === false) {
      return;
    }

    this.terminal = terminal;
    this.disposable = terminal.onData((data) => {
      void this.echo(terminal, data).catch((error) => {
        console.error("Local terminal input handler failed", error);
      });
    });
    this.scrollDisposable = terminal.onScroll(() => {
      this.scheduleExplainButtonPositionUpdate();
    });
    this.resizeDisposable = terminal.onResize(() => {
      this.scheduleExplainButtonPositionUpdate();
    });
    this.viewportElement =
      terminal.element?.querySelector<HTMLElement>(".xterm-viewport") ??
      undefined;
    this.viewportElement?.addEventListener(
      "scroll",
      this.handleViewportScroll,
      { passive: true },
    );
    this.notifyInputState();
  }

  dispose() {
    this.disposable?.dispose();
    this.disposable = undefined;
    this.scrollDisposable?.dispose();
    this.scrollDisposable = undefined;
    this.resizeDisposable?.dispose();
    this.resizeDisposable = undefined;
    this.viewportElement?.removeEventListener(
      "scroll",
      this.handleViewportScroll,
    );
    this.viewportElement = undefined;
    if (this.scrollAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.scrollAnimationFrame);
      this.scrollAnimationFrame = undefined;
    }
    this.cancelExplanationRequest();
    this.terminal = undefined;
    this.explainElements.forEach((element) => element.remove());
    this.explainElements = [];
    this.explainButtons = [];
    this.explanationRequestVersion += 1;
    document.removeEventListener("pointerdown", this.handleDocumentPointerDown);
    this.currentDialog = undefined;
  }

  async executeCommand(command: string) {
    const terminal = this.terminal;

    if (!terminal) {
      throw new Error("终端尚未初始化");
    }

    if (this.isExecuting) {
      throw new Error("已有命令正在执行");
    }

    if (this.currentInput) {
      throw new Error("请先清空当前输入");
    }

    await this.runCommand(terminal, command, {
      checkBeforeCommand: false,
      echoCommand: true,
    });
  }

  private async echo(terminal: XTerminal, data: string) {
    if (this.isExecuting) {
      return;
    }

    if (data === "\r") {
      await this.submitCommand(terminal);
      return;
    }

    if (data === "\u0003") {
      await writeTerminalAsync(terminal, "^C\r\n");
      this.currentInput = "";
      await this.writePrompt();
      this.notifyInputState();
      return;
    }

    if (data === "\u007f") {
      if (this.currentInput.length === 0) {
        return;
      }

      const { input, width } = removeLastGrapheme(this.currentInput);
      this.currentInput = input;
      terminal.write(eraseTerminalCells(width));
      this.notifyInputState();
      return;
    }

    if (data.startsWith("\x1b") || isControlCharacter(data)) {
      return;
    }

    this.currentInput += data;
    terminal.write(data);
    this.notifyInputState();
  }

  private async submitCommand(terminal: XTerminal) {
    const command = this.currentInput;

    await this.runCommand(terminal, command, {
      checkBeforeCommand: true,
      echoCommand: false,
    });
  }

  private async runCommand(
    terminal: XTerminal,
    command: string,
    options: { checkBeforeCommand: boolean; echoCommand: boolean },
  ) {
    const { checkBeforeCommand, echoCommand } = options;

    this.currentInput = "";
    this.isExecuting = true;
    this.notifyInputState();

    try {
      if (echoCommand) {
        await writeTerminalAsync(terminal, command);
      }

      await writeTerminalAsync(terminal, "\r\n\x1b[?25l");

      const isHandled = checkBeforeCommand
        ? await this.options.onBeforeCommand?.(command)
        : false;

      if (isHandled) {
        return;
      }

      if (!this.options.onCommand) {
        return;
      }

      const firstOutputRow =
        terminal.buffer.active.baseY + terminal.buffer.active.cursorY;
      let output = "";
      let pendingOutputWrite = Promise.resolve();
      const onOutput = (chunk: string) => {
        if (!chunk) {
          return;
        }

        output += chunk;
        pendingOutputWrite = pendingOutputWrite.then(() =>
          writeTerminalAsync(terminal, normalizeTerminalOutput(chunk)),
        );
      };
      const returnedOutput = await this.options.onCommand(command, onOutput);

      if (returnedOutput) {
        onOutput(returnedOutput);
      }

      await pendingOutputWrite;

      if (hasVisibleTerminalOutput(output)) {
        this.createExplainButton(command, output, firstOutputRow);
        if (terminal.buffer.active.cursorX !== 0) {
          await writeTerminalAsync(terminal, "\r\n");
        }
      }
    } finally {
      try {
        await this.writePrompt(true);
      } finally {
        this.isExecuting = false;
        this.notifyInputState();
      }
    }
  }

  private notifyInputState() {
    this.onInputStateChange?.(!this.isExecuting, this.currentInput);
  }

  private async writePrompt(showCursor = false) {
    const prompt = this.getPrompt();

    if (!prompt || !this.terminal) {
      return;
    }

    await writeTerminalAsync(
      this.terminal,
      `${prompt}${showCursor ? "\x1b[?25h" : ""}`,
    );
  }

  private getPrompt() {
    if (typeof this.options.prompt === "function") {
      return this.options.prompt();
    }

    return this.options.prompt ?? "";
  }

  private createExplainButton(
    command: string,
    output: string,
    firstOutputRow: number,
  ) {
    if (!this.options.onExplain || !this.terminal?.element) {
      return;
    }

    const anchor = this.getExplainButtonAnchor(firstOutputRow);
    const viewportAnchor = this.getExplainButtonViewportAnchor(anchor);
    const isVisible = this.isExplainButtonVisible(anchor);

    let buttonInstance: ExplainButtonInstance;
    const host = createExplainButton({
      ...viewportAnchor,
      onClick: () => {
        void this.openExplainDialog(
          command,
          output,
          this.getExplainButtonViewportAnchor(buttonInstance.anchor),
        );
      },
    });
    const button =
      host.querySelector<HTMLElement>(".ai-terminal-explain-button") ?? host;

    this.terminal.element.appendChild(host);
    button.hidden = !isVisible;
    this.explainElements.push(host);
    buttonInstance = {
      anchor,
      element: button,
      isVisible,
      left: viewportAnchor.left,
      size: viewportAnchor.size,
      top: viewportAnchor.top,
    };
    this.explainButtons.push(buttonInstance);
  }

  private getExplainButtonAnchor(firstOutputRow: number): ExplainButtonAnchor {
    if (!this.terminal) {
      return { column: 0, row: firstOutputRow };
    }

    const buffer = this.terminal.buffer.active;
    const cursorRow = buffer.baseY + buffer.cursorY;

    if (buffer.cursorX > 0) {
      return { column: buffer.cursorX + 1, row: cursorRow };
    }

    for (let row = cursorRow - 1; row >= firstOutputRow; row -= 1) {
      const line = buffer.getLine(row);
      if (!line) {
        continue;
      }

      for (let column = line.length - 1; column >= 0; column -= 1) {
        const cell = line.getCell(column);
        if (!cell?.getChars()) {
          continue;
        }

        return {
          column: column + Math.max(1, cell.getWidth()) + 1,
          row,
        };
      }
    }

    return { column: 1, row: cursorRow };
  }

  private scheduleExplainButtonPositionUpdate() {
    if (this.scrollAnimationFrame !== undefined) {
      return;
    }

    this.scrollAnimationFrame = window.requestAnimationFrame(() => {
      this.scrollAnimationFrame = undefined;
      this.updateExplainButtonPositions();
    });
  }

  private updateExplainButtonPositions() {
    if (!this.terminal) {
      return;
    }

    this.explainButtons.forEach((button) => {
      const viewportAnchor = this.getExplainButtonViewportAnchor(button.anchor);
      const isVisible = this.isExplainButtonVisible(button.anchor);

      if (button.isVisible !== isVisible) {
        button.element.hidden = !isVisible;
        button.isVisible = isVisible;
      }

      if (isVisible && button.left !== viewportAnchor.left) {
        button.element.style.setProperty(
          "--ai-terminal-explain-button-left",
          `${viewportAnchor.left}px`,
        );
        button.left = viewportAnchor.left;
      }

      if (isVisible && button.top !== viewportAnchor.top) {
        button.element.style.setProperty(
          "--ai-terminal-explain-button-top",
          `${viewportAnchor.top}px`,
        );
        button.top = viewportAnchor.top;
      }

      if (isVisible && button.size !== viewportAnchor.size) {
        button.element.style.setProperty(
          "--ai-terminal-explain-button-size",
          `${viewportAnchor.size}px`,
        );
        button.size = viewportAnchor.size;
      }
    });
  }

  private getExplainButtonViewportAnchor(
    anchor: ExplainButtonAnchor,
  ): ExplainButtonViewportAnchor {
    if (!this.terminal) {
      return {
        left: 0,
        row: anchor.row,
        top: 0,
        size: 14,
      };
    }

    const cell = getTerminalCellMetrics(this.terminal);
    const top =
      (anchor.row - this.terminal.buffer.active.viewportY) * cell.height + 1;

    return {
      left: anchor.column * cell.width,
      row: anchor.row,
      top,
      size: Math.max(14, cell.height - 2),
    };
  }

  private isExplainButtonVisible(anchor: ExplainButtonAnchor) {
    if (!this.terminal) {
      return false;
    }

    const viewportRow = anchor.row - this.terminal.buffer.active.viewportY;
    return viewportRow >= 0 && viewportRow < this.terminal.rows;
  }

  private async openExplainDialog(
    command: string,
    output: string,
    anchor: ExplainButtonViewportAnchor,
  ) {
    if (!this.options.onExplain || !this.terminal?.element) {
      return;
    }

    this.closeExplainDialog();

    const title = this.options.explainTitle ?? "命令解释";
    const position = this.getExplainDialogPosition(anchor);
    const dialog = createExplainDialog(
      {
        status: "loading",
        title,
        content: "正在生成解释...",
      },
      {
        onClose: () => {
          this.closeExplainDialog();
        },
        onRefresh: () => {
          void this.renderExplanation(dialog, command, output, position);
        },
        position,
      },
    );

    this.terminal.element.appendChild(dialog);
    this.explainElements.push(dialog);
    this.currentDialog = dialog;
    document.addEventListener("pointerdown", this.handleDocumentPointerDown);

    await this.renderExplanation(dialog, command, output, position);
  }

  private closeExplainDialog() {
    if (!this.currentDialog) {
      return;
    }

    this.explanationRequestVersion += 1;
    this.cancelExplanationRequest();
    this.currentDialog.remove();
    this.explainElements = this.explainElements.filter(
      (element) => element !== this.currentDialog,
    );
    this.currentDialog = undefined;
    document.removeEventListener("pointerdown", this.handleDocumentPointerDown);
  }

  private async renderExplanation(
    dialog: HTMLElement,
    command: string,
    output: string,
    position: ExplainDialogPosition,
  ) {
    if (!this.options.onExplain) {
      return;
    }

    this.cancelExplanationRequest();
    const requestVersion = ++this.explanationRequestVersion;
    const abortController = new AbortController();
    this.explanationAbortController = abortController;

    const title = this.options.explainTitle ?? "命令解释";
    const actions = {
      onClose: () => {
        this.closeExplainDialog();
      },
      onRefresh: () => {
        void this.renderExplanation(dialog, command, output, position);
      },
      position,
    };

    renderExplainDialog(
      dialog,
      {
        status: "loading",
        title,
        content: "正在生成解释...",
      },
      actions,
    );

    let content = "";
    let lastRenderAt = 0;
    const renderStreamingContent = () => {
      this.explanationRenderTimer = undefined;
      if (!this.isCurrentExplanation(dialog, requestVersion)) {
        return;
      }

      lastRenderAt = Date.now();
      renderExplainDialog(
        dialog,
        {
          status: "streaming",
          title,
          content,
          markdown: true,
        },
        actions,
      );
    };

    try {
      content = await this.options.onExplain(
        { command, output },
        { signal: abortController.signal },
      );

      this.clearExplanationRenderTimer();
      if (!this.isCurrentExplanation(dialog, requestVersion)) {
        return;
      }

      renderExplainDialog(
        dialog,
        {
          status: "success",
          title,
          content: content || "未生成解释内容",
          markdown: content.length > 0,
        },
        actions,
      );
    } catch (error) {
      this.clearExplanationRenderTimer();
      if (abortController.signal.aborted || isAbortError(error)) {
        return;
      }
      if (!this.isCurrentExplanation(dialog, requestVersion)) {
        return;
      }

      const message = error instanceof Error ? error.message : "解释生成失败";
      renderExplainDialog(
        dialog,
        {
          status: "error",
          title,
          content: content
            ? `${content}\n\n> 解释生成中断：${message}`
            : message,
          markdown: content.length > 0,
        },
        actions,
      );
    } finally {
      if (this.explanationAbortController === abortController) {
        this.explanationAbortController = undefined;
      }
    }
  }

  private cancelExplanationRequest() {
    this.explanationAbortController?.abort();
    this.explanationAbortController = undefined;
    this.clearExplanationRenderTimer();
  }

  private clearExplanationRenderTimer() {
    if (this.explanationRenderTimer === undefined) {
      return;
    }

    window.clearTimeout(this.explanationRenderTimer);
    this.explanationRenderTimer = undefined;
  }

  private isCurrentExplanation(dialog: HTMLElement, requestVersion: number) {
    return (
      this.currentDialog === dialog &&
      this.explanationRequestVersion === requestVersion &&
      dialog.isConnected
    );
  }

  private getExplainDialogPosition(
    anchor: ExplainButtonViewportAnchor,
  ): ExplainDialogPosition {
    const terminalElement = this.terminal?.element;
    const containerWidth = terminalElement?.clientWidth ?? EXPLAIN_DIALOG_WIDTH;
    const containerHeight =
      terminalElement?.clientHeight ?? EXPLAIN_DIALOG_MAX_HEIGHT;
    const maxLeft = Math.max(
      EXPLAIN_DIALOG_MARGIN,
      containerWidth - EXPLAIN_DIALOG_WIDTH - EXPLAIN_DIALOG_MARGIN,
    );
    const iconCenterX = anchor.left + anchor.size / 2;
    const belowTop = anchor.top + anchor.size + EXPLAIN_DIALOG_ANCHOR_OFFSET_Y;
    const belowSpace = containerHeight - belowTop - EXPLAIN_DIALOG_MARGIN;
    const shouldPlaceAbove = belowSpace < EXPLAIN_DIALOG_MAX_HEIGHT;
    const preferredTop = shouldPlaceAbove
      ? anchor.top - EXPLAIN_DIALOG_MAX_HEIGHT - EXPLAIN_DIALOG_ANCHOR_OFFSET_Y
      : belowTop;
    const maxTop = Math.max(
      EXPLAIN_DIALOG_MARGIN,
      containerHeight - EXPLAIN_DIALOG_MAX_HEIGHT - EXPLAIN_DIALOG_MARGIN,
    );

    return {
      left: this.clamp(
        iconCenterX - EXPLAIN_DIALOG_WIDTH / 2,
        EXPLAIN_DIALOG_MARGIN,
        maxLeft,
      ),
      top: this.clamp(preferredTop, EXPLAIN_DIALOG_MARGIN, maxTop),
      ...(shouldPlaceAbove ? { height: EXPLAIN_DIALOG_MAX_HEIGHT } : {}),
    };
  }

  private clamp(value: number, min: number, max: number) {
    return Math.min(Math.max(value, min), max);
  }
}

function isAbortError(error: unknown) {
  return (
    typeof error === "object" &&
    error !== null &&
    "name" in error &&
    error.name === "AbortError"
  );
}

function removeLastGrapheme(input: string) {
  const Segmenter = (
    Intl as unknown as {
      Segmenter?: new (
        locale?: string,
        options?: { granularity: "grapheme" },
      ) => { segment(value: string): Iterable<{ segment: string }> };
    }
  ).Segmenter;
  const segments = Segmenter
    ? Array.from(
        new Segmenter(undefined, { granularity: "grapheme" }).segment(input),
      )
    : Array.from(input, (segment) => ({ segment }));
  const last = segments[segments.length - 1]?.segment ?? "";

  return {
    input: input.slice(0, -last.length),
    width: getGraphemeCellWidth(last),
  };
}

function getGraphemeCellWidth(grapheme: string) {
  if (!grapheme) {
    return 0;
  }

  if (/\p{Extended_Pictographic}/u.test(grapheme)) {
    return 2;
  }

  for (const character of grapheme) {
    const codePoint = character.codePointAt(0) ?? 0;
    if (isWideCodePoint(codePoint)) {
      return 2;
    }
  }

  return 1;
}

function isWideCodePoint(codePoint: number) {
  return (
    codePoint >= 0x1100 &&
    (codePoint <= 0x115f ||
      codePoint === 0x2329 ||
      codePoint === 0x232a ||
      (codePoint >= 0x2e80 && codePoint <= 0xa4cf) ||
      (codePoint >= 0xac00 && codePoint <= 0xd7a3) ||
      (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
      (codePoint >= 0xfe10 && codePoint <= 0xfe19) ||
      (codePoint >= 0xfe30 && codePoint <= 0xfe6f) ||
      (codePoint >= 0xff00 && codePoint <= 0xff60) ||
      (codePoint >= 0xffe0 && codePoint <= 0xffe6) ||
      (codePoint >= 0x20000 && codePoint <= 0x3fffd))
  );
}

function eraseTerminalCells(width: number) {
  const backspaces = "\b".repeat(width);
  return `${backspaces}${" ".repeat(width)}${backspaces}`;
}
