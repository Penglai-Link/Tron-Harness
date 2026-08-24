import {
  Terminal as XTerminal,
  type IDecoration,
  type IDisposable,
  type ITerminalAddon,
} from "@xterm/xterm";
import type { LocalEchoAddonOptions } from "../types/localEcho";
import { createExplainButton } from "../ui/explainButton";
import { createExplainDialog, renderExplainDialog } from "../ui/explainDialog";
import {
  createChatTerminalI18n,
  type ChatTerminalI18n,
  type ChatTerminalMessageKey,
} from "../i18n";
import {
  getTerminalCellMetrics,
  hasVisibleTerminalOutput,
  isControlCharacter,
  normalizeTerminalOutput,
  writeTerminalAsync,
} from "../utils";
import type { ExplainDialogPosition } from "../types/ui";
import type { ExplainDialogOptions, ExplainDialogState } from "../types/ui";

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

interface ActiveCommand {
  cancelled: boolean;
  interactive: boolean;
  preserveRawOutput: boolean;
  pendingOutputWrite: Promise<void>;
}

interface ActiveExplainDialog {
  contentKey?: ChatTerminalMessageKey;
  element: HTMLElement;
  options: ExplainDialogOptions;
  state: ExplainDialogState;
}

interface TerminalCursorPosition {
  column: number;
  row: number;
}

interface InputCellSpan {
  end: TerminalCursorPosition;
  start: TerminalCursorPosition;
}

const EXPLAIN_DIALOG_WIDTH = 400;
const EXPLAIN_DIALOG_MAX_HEIGHT = 180;
const EXPLAIN_DIALOG_MARGIN = 12;
const EXPLAIN_DIALOG_ANCHOR_OFFSET_Y = 8;
const EXPLAIN_BUTTON_GAP = 10;
const EXPLAIN_RENDER_INTERVAL_MS = 50;
const EDITABLE_LINE_RESIZE_DEBOUNCE_MS = 100;
const COMMAND_HISTORY_LIMIT = 100;

export class LocalEchoAddon implements ITerminalAddon {
  private disposable?: IDisposable;
  private resizeDisposable?: IDisposable;
  private compositionTextarea?: HTMLTextAreaElement;
  private resizeRedrawTimer?: number;
  private explanationRequestVersion = 0;
  private explanationAbortController?: AbortController;
  private explanationRenderTimer?: number;
  private keyEventHandler?: (event: KeyboardEvent) => boolean;
  private currentInput = "";
  private commandHistory: string[] = [];
  private historyDraft = "";
  private historyIndex?: number;
  private inputCellSpans: InputCellSpan[] = [];
  private inputQueue = Promise.resolve();
  private commandInputQueue = Promise.resolve();
  private isExecuting = false;
  private activeCommand?: ActiveCommand;
  private terminal?: XTerminal;
  private renderedColumns = 80;
  private hasRenderedEditableLine = false;
  private explainElements: HTMLElement[] = [];
  private explainDecorations: IDecoration[] = [];
  private currentDialog?: HTMLElement;
  private activeExplainDialog?: ActiveExplainDialog;
  private isComposing = false;
  private compositionSpaceCount = 0;
  private shouldRestoreImeDoubleSpace = false;
  private readonly i18n: ChatTerminalI18n;
  private readonly handleCompositionStart = () => {
    this.isComposing = true;
    this.compositionSpaceCount = 0;
    this.shouldRestoreImeDoubleSpace = false;
  };
  private readonly handleCompositionEnd = () => {
    this.isComposing = false;
    this.shouldRestoreImeDoubleSpace = this.compositionSpaceCount >= 2;
    this.compositionSpaceCount = 0;
  };
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
      interactive: boolean,
    ) => void,
  ) {
    this.i18n = options.i18n ?? createChatTerminalI18n();
  }

  activate(terminal: XTerminal) {
    if (this.options.enabled === false) {
      return;
    }

    this.terminal = terminal;
    this.renderedColumns = terminal.cols;
    this.hasRenderedEditableLine = false;
    this.disposable = terminal.onData((data) => {
      const input = this.restoreImeDoubleSpace(data);
      if (this.isExecuting) {
        if (this.activeCommand?.interactive) {
          this.enqueueCommandInput(input);
        } else if (input === "\u0003") {
          this.cancelCurrentCommand();
        }
        return;
      }

      this.enqueueInput(() => this.echo(terminal, input));
    });
    this.keyEventHandler = (event) => {
      this.trackCompositionSpace(event);
      const isClearShortcut =
        event.type === "keydown" &&
        event.metaKey &&
        !event.altKey &&
        !event.ctrlKey &&
        event.key.toLowerCase() === "k";

      if (!isClearShortcut || !this.clear()) {
        return true;
      }

      event.preventDefault();
      return false;
    };
    terminal.attachCustomKeyEventHandler(this.keyEventHandler);
    if (terminal.textarea) {
      this.compositionTextarea = terminal.textarea;
      this.compositionTextarea.addEventListener(
        "compositionstart",
        this.handleCompositionStart,
      );
      this.compositionTextarea.addEventListener(
        "compositionend",
        this.handleCompositionEnd,
      );
    }
    this.resizeDisposable = terminal.onResize(() => {
      this.scheduleEditableLineRedraw(terminal);
    });
    this.notifyInputState();
  }

  dispose() {
    this.disposable?.dispose();
    this.disposable = undefined;
    this.keyEventHandler = undefined;
    this.compositionTextarea?.removeEventListener(
      "compositionstart",
      this.handleCompositionStart,
    );
    this.compositionTextarea?.removeEventListener(
      "compositionend",
      this.handleCompositionEnd,
    );
    this.compositionTextarea = undefined;
    this.isComposing = false;
    this.compositionSpaceCount = 0;
    this.shouldRestoreImeDoubleSpace = false;
    this.resizeDisposable?.dispose();
    this.resizeDisposable = undefined;
    if (this.resizeRedrawTimer !== undefined) {
      window.clearTimeout(this.resizeRedrawTimer);
      this.resizeRedrawTimer = undefined;
    }
    this.cancelExplanationRequest();
    this.terminal = undefined;
    this.hasRenderedEditableLine = false;
    this.inputCellSpans = [];
    this.inputQueue = Promise.resolve();
    this.commandInputQueue = Promise.resolve();
    this.explainElements.forEach((element) => element.remove());
    this.explainElements = [];
    this.explainDecorations.forEach((decoration) => decoration.dispose());
    this.explainDecorations = [];
    this.explanationRequestVersion += 1;
    document.removeEventListener("pointerdown", this.handleDocumentPointerDown);
    this.currentDialog = undefined;
    this.activeExplainDialog = undefined;
  }

  markPromptRendered() {
    if (!this.terminal) {
      return;
    }

    this.hasRenderedEditableLine = true;
    this.renderedColumns = this.terminal.cols;
    this.inputCellSpans = [];
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

  refreshLocale() {
    if (!this.activeExplainDialog) {
      return;
    }

    this.renderActiveExplainDialog(this.activeExplainDialog);
  }

  clear() {
    const terminal = this.terminal;

    if (!terminal || this.isExecuting) {
      return false;
    }

    this.enqueueInput(() => this.clearTerminal(terminal));
    return true;
  }

  private async echo(terminal: XTerminal, data: string) {
    if (this.isExecuting) {
      if (data === "\u0003") {
        this.cancelCurrentCommand();
      }
      return;
    }

    if (data === "\r") {
      await this.submitCommand(terminal);
      return;
    }

    if (data === "\x1b[A" || data === "\x1bOA") {
      await this.navigateCommandHistory(terminal, -1);
      return;
    }

    if (data === "\x1b[B" || data === "\x1bOB") {
      await this.navigateCommandHistory(terminal, 1);
      return;
    }

    if (data === "\u0003") {
      await writeTerminalAsync(terminal, "^C\r\n");
      this.currentInput = "";
      this.inputCellSpans = [];
      this.clearHistoryNavigation();
      await this.writePrompt();
      this.notifyInputState();
      return;
    }

    if (data === "\u007f") {
      if (this.currentInput.length === 0) {
        return;
      }

      this.clearHistoryNavigation();
      const shouldRedraw = this.currentInput.includes("\n");
      const { input, width } = removeLastGrapheme(this.currentInput);
      this.currentInput = input;
      if (shouldRedraw) {
        await this.redrawEditableLine(terminal);
        this.notifyInputState();
        return;
      }
      const span = this.inputCellSpans.pop();
      if (span) {
        await writeTerminalAsync(terminal, eraseTerminalSpan(terminal, span));
      } else {
        await writeTerminalAsync(terminal, eraseTerminalCells(width));
      }
      this.notifyInputState();
      return;
    }

    if (data.startsWith("\x1b") || isControlCharacter(data)) {
      return;
    }

    const input = normalizeTerminalInput(data);
    const graphemes = segmentGraphemes(input);
    this.clearHistoryNavigation();
    const start = getTerminalCursorPosition(terminal);
    this.inputCellSpans.push(
      ...createInputCellSpans(start, graphemes, terminal.cols),
    );
    this.currentInput += graphemes.join("");
    this.notifyInputState();
    await writeTerminalAsync(terminal, normalizeTerminalOutput(input));
    this.renderedColumns = terminal.cols;
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
    const submittedCommandEnd = echoCommand
      ? undefined
      : getTerminalCursorPosition(terminal);
    const interactive = Boolean(
      this.options.onCommandInput &&
        (this.options.isCommandInputEnabled?.(command) ?? true),
    );
    const preserveRawOutput = Boolean(
      this.options.isInteractiveCommand?.(command),
    );

    this.currentInput = "";
    this.inputCellSpans = [];
    this.clearHistoryNavigation();
    this.isExecuting = true;
    const activeCommand: ActiveCommand = {
      cancelled: false,
      interactive,
      preserveRawOutput,
      pendingOutputWrite: Promise.resolve(),
    };
    this.activeCommand = activeCommand;
    terminal.element?.classList.toggle(
      "ai-terminal-interactive-mode",
      preserveRawOutput,
    );
    this.notifyInputState();

    try {
      if (echoCommand) {
        await writeTerminalAsync(terminal, command);
      }

      const commandEnd =
        submittedCommandEnd ?? getTerminalCursorPosition(terminal);

      await writeTerminalAsync(
        terminal,
        interactive ? "\r\n\x1b[?25h" : "\r\n\x1b[?25l",
      );

      const isHandled = checkBeforeCommand
        ? await this.options.onBeforeCommand?.(command)
        : false;

      if (isHandled) {
        return;
      }

      if (!this.options.onCommand) {
        return;
      }

      this.recordCommand(command);

      let output = "";
      const onOutput = (chunk: string) => {
        if (!chunk || activeCommand.cancelled) {
          return;
        }

        output += chunk;
        activeCommand.pendingOutputWrite =
          activeCommand.pendingOutputWrite.then(() =>
            writeTerminalAsync(
              terminal,
              activeCommand.preserveRawOutput
                ? chunk
                : normalizeTerminalOutput(chunk),
            ),
          );
      };
      const returnedOutput = await this.options.onCommand(command, onOutput, {
        interactive,
      });

      if (returnedOutput && !activeCommand.cancelled) {
        onOutput(returnedOutput);
      }

      await activeCommand.pendingOutputWrite;

      if (activeCommand.cancelled) {
        await writeTerminalAsync(terminal, "^C\r\n");
        return;
      }

      if (!activeCommand.preserveRawOutput && hasVisibleTerminalOutput(output)) {
        this.createExplainButton(command, output, commandEnd);
        if (terminal.buffer.active.cursorX !== 0) {
          await writeTerminalAsync(terminal, "\r\n");
        }
      }
    } finally {
      try {
        await this.writePrompt(true);
      } finally {
        terminal.element?.classList.remove("ai-terminal-interactive-mode");
        if (this.activeCommand === activeCommand) {
          this.activeCommand = undefined;
        }
        this.isExecuting = false;
        this.notifyInputState();
      }
    }
  }

  private cancelCurrentCommand() {
    const activeCommand = this.activeCommand;

    if (!activeCommand || activeCommand.cancelled) {
      return;
    }

    activeCommand.cancelled = true;
    this.options.onCancelCommand?.();
  }

  private async navigateCommandHistory(terminal: XTerminal, direction: -1 | 1) {
    if (this.commandHistory.length === 0) {
      return;
    }

    if (direction === -1) {
      if (this.historyIndex === undefined) {
        this.historyDraft = this.currentInput;
        this.historyIndex = this.commandHistory.length - 1;
      } else {
        this.historyIndex = Math.max(0, this.historyIndex - 1);
      }
    } else if (this.historyIndex === undefined) {
      return;
    } else if (this.historyIndex < this.commandHistory.length - 1) {
      this.historyIndex += 1;
    } else {
      const draft = this.historyDraft;
      this.clearHistoryNavigation();
      await this.replaceCurrentInput(terminal, draft);
      return;
    }

    await this.replaceCurrentInput(
      terminal,
      this.commandHistory[this.historyIndex] ?? "",
    );
  }

  private recordCommand(command: string) {
    if (!command.trim()) {
      return;
    }

    this.commandHistory.push(command);
    if (this.commandHistory.length > COMMAND_HISTORY_LIMIT) {
      this.commandHistory.shift();
    }
  }

  private clearHistoryNavigation() {
    this.historyDraft = "";
    this.historyIndex = undefined;
  }

  private async replaceCurrentInput(terminal: XTerminal, input: string) {
    this.currentInput = input;
    await this.redrawEditableLine(terminal);
    this.notifyInputState();
  }

  private async clearTerminal(terminal: XTerminal) {
    terminal.clear();

    const prompt = this.getPrompt();
    const inputStart = advanceTextCursor(
      { column: 0, row: 0 },
      prompt,
      terminal.cols,
    );
    this.inputCellSpans = createInputCellSpans(
      inputStart,
      segmentGraphemes(this.currentInput),
      terminal.cols,
    );
    await writeTerminalAsync(
      terminal,
      `\r\x1b[2K${prompt}${normalizeTerminalOutput(this.currentInput)}`,
    );
    this.hasRenderedEditableLine = true;
    this.renderedColumns = terminal.cols;
    this.notifyInputState();
  }

  private enqueueInput(task: () => Promise<void> | void | undefined) {
    this.inputQueue = this.inputQueue.then(task).catch((error) => {
      console.error("Local terminal input handler failed", error);
    });
  }

  private enqueueCommandInput(data: string) {
    this.commandInputQueue = this.commandInputQueue
      .then(() => this.options.onCommandInput?.(data))
      .catch((error) => {
        console.error("Interactive terminal input handler failed", error);
      });
  }

  private notifyInputState() {
    this.onInputStateChange?.(
      !this.isExecuting,
      this.currentInput,
      this.activeCommand?.interactive ?? false,
    );
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
    this.hasRenderedEditableLine = true;
    this.renderedColumns = this.terminal.cols;
  }

  private async redrawEditableLine(terminal: XTerminal) {
    if (
      this.isExecuting ||
      !this.hasRenderedEditableLine ||
      this.terminal !== terminal
    ) {
      return;
    }

    const prompt = this.getPrompt();
    const rowsToClear = getRenderedRowCount(
      `${prompt}${this.currentInput}`,
      this.renderedColumns,
    );
    const current = getTerminalCursorPosition(terminal);
    const inputStart = advanceTextCursor(
      {
        column: 0,
        row: current.row - rowsToClear + 1,
      },
      prompt,
      terminal.cols,
    );
    this.inputCellSpans = createInputCellSpans(
      inputStart,
      segmentGraphemes(this.currentInput),
      terminal.cols,
    );

    await writeTerminalAsync(
      terminal,
      `${eraseTerminalRows(rowsToClear)}${prompt}${normalizeTerminalOutput(this.currentInput)}`,
    );
    this.renderedColumns = terminal.cols;
  }

  private scheduleEditableLineRedraw(terminal: XTerminal) {
    if (this.resizeRedrawTimer !== undefined) {
      window.clearTimeout(this.resizeRedrawTimer);
    }

    this.resizeRedrawTimer = window.setTimeout(() => {
      this.resizeRedrawTimer = undefined;
      if (!this.isExecuting) {
        this.enqueueInput(() => this.redrawEditableLine(terminal));
      }
    }, EDITABLE_LINE_RESIZE_DEBOUNCE_MS);
  }

  private getPrompt() {
    if (typeof this.options.prompt === "function") {
      return this.options.prompt();
    }

    return this.options.prompt ?? "";
  }

  private trackCompositionSpace(event: KeyboardEvent) {
    if (event.type !== "keydown" || !this.isComposing) {
      return;
    }

    if (event.key === " " || event.code === "Space") {
      this.compositionSpaceCount += 1;
      return;
    }

    if (!event.metaKey && !event.altKey && !event.ctrlKey && event.key !== "Shift") {
      this.compositionSpaceCount = 0;
    }
  }

  private restoreImeDoubleSpace(data: string) {
    if (!this.shouldRestoreImeDoubleSpace) {
      return data;
    }

    this.shouldRestoreImeDoubleSpace = false;
    return [".", ". ", "。"].includes(data) ? "  " : data;
  }

  private createExplainButton(
    command: string,
    output: string,
    commandEnd: TerminalCursorPosition,
  ) {
    if (!this.options.onExplain || !this.terminal?.element) {
      return;
    }

    const anchor = this.getExplainButtonAnchor(commandEnd);
    const terminal = this.terminal;
    const currentRow =
      terminal.buffer.active.baseY + terminal.buffer.active.cursorY;
    const marker = terminal.registerMarker(anchor.row - currentRow);
    const decoration = terminal.registerDecoration({
      marker,
      anchor: "left",
      x: anchor.column,
      width: 1,
      height: 1,
      layer: "top",
    });

    if (!decoration) {
      marker.dispose();
      return;
    }

    const host = createExplainButton({
      left: EXPLAIN_BUTTON_GAP,
      top: 1,
      size: Math.max(14, getTerminalCellMetrics(terminal).height - 2),
      onClick: () => {
        void this.openExplainDialog(
          command,
          output,
          this.getExplainButtonViewportAnchor({
            column: anchor.column,
            row: decoration.marker.line,
          }, EXPLAIN_BUTTON_GAP),
        );
      },
    });

    const mountExplainButton = (element: HTMLElement) => {
      element.style.pointerEvents = "auto";
      element.style.overflow = "visible";
      if (host.parentElement !== element) {
        element.appendChild(host);
      }

      host
        .querySelector<HTMLElement>(".ai-terminal-explain-button")
        ?.style.setProperty(
          "--ai-terminal-explain-button-size",
          `${Math.max(14, getTerminalCellMetrics(terminal).height - 2)}px`,
        );
    };
    decoration.onRender(mountExplainButton);
    if (decoration.element) {
      mountExplainButton(decoration.element);
    }
    decoration.onDispose(() => host.remove());
    this.explainDecorations.push(decoration);
  }

  private getExplainButtonAnchor(
    commandEnd: TerminalCursorPosition,
  ): ExplainButtonAnchor {
    return { column: commandEnd.column, row: commandEnd.row };
  }

  private getExplainButtonViewportAnchor(
    anchor: ExplainButtonAnchor,
    horizontalOffset = 0,
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
      left: anchor.column * cell.width + horizontalOffset,
      row: anchor.row,
      top,
      size: Math.max(14, cell.height - 2),
    };
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

    const position = this.getExplainDialogPosition(anchor);
    const dialogState: ExplainDialogState = {
      status: "loading",
      title: "",
      content: "",
    };
    const dialogOptions: ExplainDialogOptions = {
      onClose: () => {
        this.closeExplainDialog();
      },
      onRefresh: () => {
        void this.renderExplanation(dialog, command, output, position);
      },
      position,
    };
    const dialog = createExplainDialog(
      this.localizeExplainState(dialogState, "explainLoading"),
      dialogOptions,
    );

    this.terminal.element.appendChild(dialog);
    this.explainElements.push(dialog);
    this.currentDialog = dialog;
    this.activeExplainDialog = {
      contentKey: "explainLoading",
      element: dialog,
      options: dialogOptions,
      state: dialogState,
    };
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
    this.activeExplainDialog = undefined;
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

    const actions = {
      onClose: () => {
        this.closeExplainDialog();
      },
      onRefresh: () => {
        void this.renderExplanation(dialog, command, output, position);
      },
      position,
    };
    const loadingState: ExplainDialogState = {
      status: "loading",
      title: "",
      content: "",
    };
    this.activeExplainDialog = {
      contentKey: "explainLoading",
      element: dialog,
      options: actions,
      state: loadingState,
    };

    this.renderActiveExplainDialog(this.activeExplainDialog);

    let content = "";
    let lastRenderAt = 0;
    const renderStreamingContent = () => {
      this.explanationRenderTimer = undefined;
      if (!this.isCurrentExplanation(dialog, requestVersion)) {
        return;
      }

      lastRenderAt = Date.now();
      this.activeExplainDialog = {
        element: dialog,
        options: actions,
        state: {
          status: "streaming",
          title: "",
          content,
          markdown: true,
        },
      };
      this.renderActiveExplainDialog(this.activeExplainDialog);
    };

    try {
      const finalContent = await this.options.onExplain(
        { command, output },
        {
          onChunk: (delta) => {
            if (
              !delta ||
              abortController.signal.aborted ||
              !this.isCurrentExplanation(dialog, requestVersion)
            ) {
              return;
            }

            content += delta;
            const renderDelay = Math.max(
              0,
              EXPLAIN_RENDER_INTERVAL_MS - (Date.now() - lastRenderAt),
            );
            if (renderDelay === 0) {
              renderStreamingContent();
            } else if (this.explanationRenderTimer === undefined) {
              this.explanationRenderTimer = window.setTimeout(
                renderStreamingContent,
                renderDelay,
              );
            }
          },
          signal: abortController.signal,
        },
      );
      content = finalContent || content;

      this.clearExplanationRenderTimer();
      if (!this.isCurrentExplanation(dialog, requestVersion)) {
        return;
      }

      this.activeExplainDialog = {
        contentKey: content ? undefined : "explainEmpty",
        element: dialog,
        options: actions,
        state: {
          status: "success",
          title: "",
          content,
          markdown: content.length > 0,
        },
      };
      this.renderActiveExplainDialog(this.activeExplainDialog);
    } catch (error) {
      this.clearExplanationRenderTimer();
      if (abortController.signal.aborted || isAbortError(error)) {
        return;
      }
      if (!this.isCurrentExplanation(dialog, requestVersion)) {
        return;
      }

      const message = error instanceof Error ? error.message : undefined;
      this.activeExplainDialog = {
        contentKey: message || content ? undefined : "explainFailed",
        element: dialog,
        options: actions,
        state: {
          status: "error",
          title: "",
          content: content
            ? `${content}\n\n> ${this.i18n.t("explainInterrupted")}：${
                message ?? this.i18n.t("explainFailed")
              }`
            : (message ?? ""),
          markdown: content.length > 0,
        },
      };
      this.renderActiveExplainDialog(this.activeExplainDialog);
    } finally {
      if (this.explanationAbortController === abortController) {
        this.explanationAbortController = undefined;
      }
    }
  }

  private renderActiveExplainDialog(dialog: ActiveExplainDialog) {
    renderExplainDialog(
      dialog.element,
      this.localizeExplainState(dialog.state, dialog.contentKey),
      dialog.options,
    );
  }

  private localizeExplainState(
    state: ExplainDialogState,
    contentKey?: ChatTerminalMessageKey,
  ): ExplainDialogState {
    return {
      ...state,
      title: this.options.explainTitle ?? this.i18n.t("explainTitle"),
      content: contentKey ? this.i18n.t(contentKey) : state.content,
    };
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

export function isInteractiveShellCommand(command: string) {
  const normalized = command.trim().replace(/^(?:command|env|sudo)\s+/, "");

  return /^(?:\S*\/)?(?:vim|vi|nvim|nano|emacs|top|htop|less|more|man|watch|tmux|screen)(?:\s|$)/.test(
    normalized,
  );
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
  const segments = segmentGraphemes(input).map((segment) => ({ segment }));
  const last = segments[segments.length - 1]?.segment ?? "";

  return {
    input: input.slice(0, -last.length),
    width: getGraphemeCellWidth(last),
  };
}

function segmentGraphemes(input: string) {
  const Segmenter = (
    Intl as unknown as {
      Segmenter?: new (
        locale?: string,
        options?: { granularity: "grapheme" },
      ) => { segment(value: string): Iterable<{ segment: string }> };
    }
  ).Segmenter;

  return Segmenter
    ? Array.from(
        new Segmenter(undefined, { granularity: "grapheme" }).segment(input),
        ({ segment }) => segment,
      )
    : Array.from(input);
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

function getTerminalCursorPosition(terminal: XTerminal): TerminalCursorPosition {
  const buffer = terminal.buffer.active;

  return {
    column: buffer.cursorX,
    row: buffer.baseY + buffer.cursorY,
  };
}

function advanceTerminalCursor(
  position: TerminalCursorPosition,
  grapheme: string,
  columns: number,
) {
  if (grapheme === "\n") {
    return {
      column: 0,
      row: position.row + 1,
    };
  }

  if (grapheme === "\t") {
    if (position.column >= columns) {
      return position;
    }

    return {
      column: Math.min(
        columns - 1,
        position.column + (8 - (position.column % 8)),
      ),
      row: position.row,
    };
  }

  let row = position.row;
  let column = position.column;
  const width = getGraphemeCellWidth(grapheme);

  if (column >= columns || (width > 1 && column === columns - 1)) {
    row += 1;
    column = 0;
  }

  return {
    column: column + width,
    row,
  };
}

function normalizeTerminalInput(data: string) {
  return data.replace(/\r\n?|\n/g, "\n");
}

function createInputCellSpans(
  start: TerminalCursorPosition,
  graphemes: string[],
  columns: number,
) {
  const spans: InputCellSpan[] = [];
  let cursor = start;

  for (const grapheme of graphemes) {
    const end = advanceTerminalCursor(cursor, grapheme, columns);
    spans.push({ start: cursor, end });
    cursor = end;
  }

  return spans;
}

function getRenderedRowCount(text: string, columns: number) {
  return advanceTextCursor(
    { column: 0, row: 0 },
    text,
    columns,
  ).row + 1;
}

function advanceTextCursor(
  start: TerminalCursorPosition,
  text: string,
  columns: number,
) {
  let cursor = start;

  for (const grapheme of segmentGraphemes(text)) {
    cursor = advanceTerminalCursor(cursor, grapheme, columns);
  }

  return cursor;
}

function eraseTerminalRows(rowCount: number) {
  let sequence = "\r\x1b[2K";

  for (let row = 1; row < rowCount; row += 1) {
    sequence += "\x1b[1A\r\x1b[2K";
  }

  return sequence;
}

function eraseTerminalSpan(terminal: XTerminal, span: InputCellSpan) {
  const columns = terminal.cols;
  const start = normalizeRenderedStart(span.start, columns);
  const width = getTerminalSpanWidth(start, span.end, columns);

  if (width <= 0) {
    return "";
  }

  const current = getTerminalCursorPosition(terminal);
  let sequence = moveTerminalCursor(current, start);
  let remaining = width;
  let cursor = start;

  while (remaining > 0) {
    const cellsOnLine = Math.max(1, columns - cursor.column);
    const cellsToErase = Math.min(remaining, cellsOnLine);

    sequence += `\x1b[${cellsToErase}X`;
    remaining -= cellsToErase;

    if (remaining > 0) {
      sequence += "\x1b[1B\r";
      cursor = { column: 0, row: cursor.row + 1 };
    }
  }

  return sequence + moveTerminalCursor(cursor, start);
}

function normalizeRenderedStart(
  position: TerminalCursorPosition,
  columns: number,
) {
  if (position.column < columns) {
    return position;
  }

  return {
    column: 0,
    row: position.row + 1,
  };
}

function getTerminalSpanWidth(
  start: TerminalCursorPosition,
  end: TerminalCursorPosition,
  columns: number,
) {
  const endColumn = Math.min(end.column, columns);
  return (end.row - start.row) * columns + endColumn - start.column;
}

function moveTerminalCursor(
  from: TerminalCursorPosition,
  to: TerminalCursorPosition,
) {
  const rowDelta = to.row - from.row;
  const columnDelta = to.column - from.column;
  let sequence = "";

  if (rowDelta < 0) {
    sequence += `\x1b[${-rowDelta}A`;
  } else if (rowDelta > 0) {
    sequence += `\x1b[${rowDelta}B`;
  }

  if (columnDelta < 0) {
    sequence += `\x1b[${-columnDelta}D`;
  } else if (columnDelta > 0) {
    sequence += `\x1b[${columnDelta}C`;
  }

  return sequence;
}
