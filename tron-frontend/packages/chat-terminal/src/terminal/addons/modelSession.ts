import {
  Terminal as XTerminal,
  type IDisposable,
  type IMarker,
  type ITerminalAddon,
} from "@xterm/xterm";
import {
  computed,
  effect,
  signal,
  type ReadonlySignal,
  type Signal,
} from "@preact/signals-core";
import { createModelSessionHost, renderModelSession } from "../ui/modelSession";
import { getTerminalCellMetrics, writeTerminalAsync } from "../utils";
import { createChatTerminalI18n, type ChatTerminalI18n } from "../i18n";
import type {
  ModelSessionActivityEvent,
  ModelSessionAddonOptions,
  ModelSessionCommandActionPayload,
  ModelSessionCommandCardState,
  ModelSessionCommandToken,
  ModelSessionMessageChunk,
  ModelSessionRenderActions,
  ModelSessionToken,
} from "../types/modelSession";
import type { AIStreamEvent } from "../../transport/types";

interface ParsedSsePayload {
  requestId: string;
  conversationId: string;
  response: string;
}

interface BlockParserResult {
  index: number;
  token: ModelSessionCommandToken;
  endIndex: number;
}

interface BlockParser {
  findStart(input: string, startIndex: number): number;
  parse(input: string, startIndex: number): BlockParserResult | null;
}

interface ModelSessionInstance {
  key: string;
  requestId: string;
  conversationId: string;
  rawMessage: Signal<string>;
  activityEvents: Signal<readonly AIStreamEvent[]>;
  activityStartedAt: number;
  activityNow: Signal<number>;
  activityComplete: Signal<boolean>;
  activityClock?: ReturnType<typeof setInterval>;
  tokens: ReadonlySignal<ModelSessionToken[]>;
  anchorMarker: IMarker;
  reservedRows: number;
  fixedHeight?: number;
  host: HTMLElement;
  commandStates: Signal<ReadonlyMap<string, ModelSessionCommandCardState>>;
  actions: ModelSessionRenderActions;
  renderAnimationFrame?: number;
  disposeRenderEffect: () => void;
}

const COMMAND_PARSERS: BlockParser[] = [
  {
    findStart: findNextExecuteCommandStart,
    parse: parseExecuteCommandBlock,
  },
];
const MODEL_SESSION_ROOT_CLASS = "ai-terminal-model-session-root";

export class ModelSessionAddon implements ITerminalAddon {
  private terminal?: XTerminal;
  private scrollDisposable?: IDisposable;
  private resizeDisposable?: IDisposable;
  private viewportElement?: HTMLElement;
  private sessionResizeObserver?: ResizeObserver;
  private readonly sessionWidths = new Map<string, number>();
  private positionAnimationFrame?: number;
  private shouldMeasureSessions = false;
  private sseBuffer = "";
  private readonly sessions = new Map<string, ModelSessionInstance>();
  private activeSessionKey?: string;
  private readonly isVisible: Signal<boolean>;
  private readonly i18n: ChatTerminalI18n;
  private readonly handleViewportScroll = () => {
    this.scheduleInlineLayoutUpdate();
  };
  private readonly handleSessionToggle = (event: Event) => {
    if (!(event.target instanceof HTMLDetailsElement)) {
      return;
    }

    const host = event.currentTarget as HTMLElement;
    const session = this.sessions.get(host.dataset.sessionKey ?? "");
    if (!session) {
      return;
    }

    if (!session.activityComplete.peek()) {
      return;
    }

    this.options.onLayoutResize?.();
  };

  constructor(private readonly options: ModelSessionAddonOptions = {}) {
    this.isVisible = signal(options.visible ?? false);
    this.i18n = options.i18n ?? createChatTerminalI18n();
  }

  activate(terminal: XTerminal) {
    if (this.options.enabled === false) {
      return;
    }

    if (!terminal.element) {
      throw new Error(
        "ModelSessionAddon must be loaded after terminal.open().",
      );
    }

    this.terminal = terminal;
    terminal.element.classList.add(MODEL_SESSION_ROOT_CLASS);
    this.scrollDisposable = terminal.onScroll(() => {
      this.scheduleInlineLayoutUpdate();
    });
    this.resizeDisposable = terminal.onResize(() => {
      this.scheduleInlineLayoutUpdate(this.hasCompletedSessions());
    });
    this.viewportElement =
      terminal.element.querySelector<HTMLElement>(".xterm-viewport") ??
      undefined;
    this.viewportElement?.addEventListener(
      "scroll",
      this.handleViewportScroll,
      { passive: true },
    );
    this.sessionResizeObserver = this.createSessionResizeObserver();
  }

  dispose() {
    this.scrollDisposable?.dispose();
    this.scrollDisposable = undefined;
    this.resizeDisposable?.dispose();
    this.resizeDisposable = undefined;
    this.viewportElement?.removeEventListener(
      "scroll",
      this.handleViewportScroll,
    );
    this.viewportElement = undefined;
    this.sessionResizeObserver?.disconnect();
    this.sessionResizeObserver = undefined;
    this.sessionWidths.clear();
    if (this.positionAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.positionAnimationFrame);
      this.positionAnimationFrame = undefined;
    }
    this.sessions.forEach((session) => this.disposeSession(session));
    this.sessions.clear();
    this.terminal?.element?.classList.remove(MODEL_SESSION_ROOT_CLASS);
    this.terminal = undefined;
    this.sseBuffer = "";
    this.activeSessionKey = undefined;
    this.shouldMeasureSessions = false;
  }

  refreshLocale() {
    this.sessions.forEach((session) => {
      this.scheduleSessionRender(session);
    });
    this.scheduleInlineLayoutUpdate(true);
  }

  appendSseChunk(chunk: string) {
    this.sseBuffer += chunk.replace(/\r\n/g, "\n");

    let frameEnd = this.sseBuffer.indexOf("\n\n");
    while (frameEnd !== -1) {
      const frame = this.sseBuffer.slice(0, frameEnd).trim();
      this.sseBuffer = this.sseBuffer.slice(frameEnd + 2);
      this.consumeSseFrame(frame);
      frameEnd = this.sseBuffer.indexOf("\n\n");
    }
  }

  appendMessageChunk(chunk: ModelSessionMessageChunk) {
    const session = this.getOrCreateSession(chunk);
    if (!session) {
      return;
    }

    session.rawMessage.value += chunk.response ?? "";
    this.show();
  }

  appendActivityEvent(chunk: ModelSessionActivityEvent) {
    const session = this.getOrCreateSession({ ...chunk, response: "" });
    if (!session) return;

    session.activityEvents.value = [...session.activityEvents.peek(), chunk.event];
    this.show();
  }

  async complete(requestId?: string) {
    const completedSessions: ModelSessionInstance[] = [];

    if (!requestId) {
      const activeSession = this.activeSessionKey
        ? this.sessions.get(this.activeSessionKey)
        : undefined;
      if (activeSession) {
        completedSessions.push(activeSession);
      }
    } else {
      this.sessions.forEach((session) => {
        if (session.requestId === requestId) {
          completedSessions.push(session);
        }
      });
    }

    for (const session of completedSessions) {
      session.activityComplete.value = true;
      session.activityNow.value = Date.now();
      this.stopActivityClock(session);
      this.flushSessionRender(session);
      this.lockSessionHeight(session);
      await this.reserveSessionRows(session);
    }
  }

  clear() {
    this.sseBuffer = "";
    this.sessions.forEach((session) => this.disposeSession(session));
    this.sessions.clear();
    this.activeSessionKey = undefined;
    this.isVisible.value = false;
  }

  show() {
    if (this.isVisible.peek()) {
      return;
    }

    this.isVisible.value = true;
    this.sessions.forEach((session) => {
      session.host.hidden = false;
    });
    this.scheduleInlineLayoutUpdate(this.hasCompletedSessions());
  }

  hide() {
    if (!this.isVisible.peek()) {
      return;
    }

    this.isVisible.value = false;
    this.sessions.forEach((session) => {
      session.host.hidden = true;
    });
  }

  private consumeSseFrame(frame: string) {
    if (!frame) {
      return;
    }

    const dataLine = frame
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line.startsWith("data:"));

    if (!dataLine) {
      return;
    }

    const jsonText = dataLine.slice(5).trim();
    if (!jsonText) {
      return;
    }

    try {
      const payload = JSON.parse(jsonText) as unknown;
      const chunk = normalizeSsePayload(payload);

      if (chunk) {
        this.appendMessageChunk(chunk);
      }
    } catch (error) {
      console.warn("Failed to parse model session SSE chunk", error);
    }
  }

  private getOrCreateSession(chunk: ModelSessionMessageChunk) {
    if (!this.terminal?.element) {
      return undefined;
    }

    const key = createSessionKey(chunk.requestId, chunk.conversationId);
    const existing = this.sessions.get(key);
    if (existing) {
      this.activeSessionKey = key;
      return existing;
    }

    const buffer = this.terminal.buffer.active;
    const host = createModelSessionHost();
    const rawMessage = signal("");
    const activityEvents = signal<readonly AIStreamEvent[]>([]);
    const activityStartedAt = Date.now();
    const activityNow = signal(activityStartedAt);
    const activityComplete = signal(false);
    const commandStates = signal<
      ReadonlyMap<string, ModelSessionCommandCardState>
    >(new Map());
    const tokens = computed(() =>
      this.parseTokens(rawMessage.value, chunk.requestId, chunk.conversationId),
    );
    let session: ModelSessionInstance;
    const actions: ModelSessionRenderActions = {
      onExecuteCommand: (token) => {
        void this.executeCommand(session, token);
      },
      onStartEditCommand: (token, editorHeight) => {
        this.startEditCommand(session, token, editorHeight);
      },
      onDraftCommandChange: (token, command) => {
        this.updateDraftCommand(session, token, command);
      },
      onSaveCommandEdit: (token) => {
        void this.saveCommandEdit(session, token);
      },
      onGenerateCommand: this.options.onGenerateCommand
        ? (token) => {
            void this.generateCommand(session, token);
          }
        : undefined,
      onCancelCommandEdit: (token) => {
        this.cancelCommandEdit(session, token);
      },
      onRejectCommand: (token) => {
        void this.rejectCommand(session, token);
      },
    };
    session = {
      key,
      requestId: chunk.requestId,
      conversationId: chunk.conversationId,
      rawMessage,
      activityEvents,
      activityStartedAt,
      activityNow,
      activityComplete,
      tokens,
      anchorMarker: this.terminal.registerMarker(0),
      reservedRows: 0,
      host,
      commandStates,
      actions,
      disposeRenderEffect: () => {},
    };

    host.dataset.sessionKey = key;
    host.hidden = !this.isVisible.peek();
    this.terminal.element.appendChild(host);
    this.observeSessionHost(key, host);
    this.sessions.set(key, session);
    this.activeSessionKey = key;
    session.activityClock = setInterval(() => {
      if (!session.activityComplete.peek()) {
        session.activityNow.value = Date.now();
      }
    }, 1_000);
    session.disposeRenderEffect = effect(() => {
      tokens.value;
      activityEvents.value;
      activityNow.value;
      activityComplete.value;
      commandStates.value;
      this.scheduleSessionRender(session);
    });
    return session;
  }

  private scheduleSessionRender(session: ModelSessionInstance) {
    if (session.renderAnimationFrame !== undefined) {
      return;
    }

    session.renderAnimationFrame = window.requestAnimationFrame(() => {
      session.renderAnimationFrame = undefined;
      this.renderSession(session);
    });
  }

  private flushSessionRender(session: ModelSessionInstance) {
    if (session.renderAnimationFrame !== undefined) {
      window.cancelAnimationFrame(session.renderAnimationFrame);
      session.renderAnimationFrame = undefined;
    }

    this.renderSession(session);
  }

  private renderSession(session: ModelSessionInstance) {
    session.host.hidden = !this.isVisible.peek();

    renderModelSession(
      session.host,
      {
        tokens: session.tokens.peek(),
        activityEvents: session.activityEvents.peek(),
        activityStartedAt: session.activityStartedAt,
        activityNow: session.activityNow.peek(),
        activityComplete: session.activityComplete.peek(),
        commandStates: session.commandStates.peek(),
        i18n: this.i18n,
      },
      session.actions,
    );
    this.scheduleInlineLayoutUpdate(session.activityComplete.peek());
  }

  private disposeSession(session: ModelSessionInstance) {
    this.stopActivityClock(session);
    session.disposeRenderEffect();

    if (session.renderAnimationFrame !== undefined) {
      window.cancelAnimationFrame(session.renderAnimationFrame);
      session.renderAnimationFrame = undefined;
    }

    this.sessionResizeObserver?.unobserve(session.host);
    session.host.removeEventListener("toggle", this.handleSessionToggle, true);
    this.sessionWidths.delete(session.key);
    session.anchorMarker.dispose();
    session.host.remove();
  }

  private createSessionResizeObserver() {
    if (typeof ResizeObserver === "undefined") {
      return undefined;
    }

    return new ResizeObserver((entries) => {
      let hasWidthChange = false;

      entries.forEach((entry) => {
        const key = entry.target.getAttribute("data-session-key");
        if (!key) return;

        const previousWidth = this.sessionWidths.get(key);
        const nextWidth = entry.contentRect.width;
        this.sessionWidths.set(key, nextWidth);
        hasWidthChange ||= previousWidth !== undefined && previousWidth !== nextWidth;
      });

      if (!hasWidthChange) return;

      this.options.onLayoutResize?.();
      this.scheduleInlineLayoutUpdate(this.hasCompletedSessions());
    });
  }

  private observeSessionHost(key: string, host: HTMLElement) {
    this.sessionWidths.set(key, host.getBoundingClientRect().width);
    this.sessionResizeObserver?.observe(host);
    host.addEventListener("toggle", this.handleSessionToggle, true);
  }

  private hasCompletedSessions() {
    return [...this.sessions.values()].some((session) =>
      session.activityComplete.peek(),
    );
  }

  private stopActivityClock(session: ModelSessionInstance) {
    if (session.activityClock === undefined) return;
    clearInterval(session.activityClock);
    session.activityClock = undefined;
  }

  private scheduleInlineLayoutUpdate(shouldMeasure = false) {
    this.shouldMeasureSessions ||= shouldMeasure;

    if (this.positionAnimationFrame !== undefined) {
      return;
    }

    this.positionAnimationFrame = window.requestAnimationFrame(() => {
      this.positionAnimationFrame = undefined;
      const measureSessions = this.shouldMeasureSessions;
      this.shouldMeasureSessions = false;
      this.updateInlineLayout(measureSessions);
    });
  }

  private updateInlineLayout(shouldMeasure: boolean) {
    if (!this.terminal || !this.isVisible.peek() || this.sessions.size === 0) {
      return;
    }

    const cell = getTerminalCellMetrics(this.terminal);
    const viewportY = this.terminal.buffer.active.viewportY;
    let rowsAdded = 0;

    if (shouldMeasure) {
      this.sessions.forEach((session) => {
        if (!session.activityComplete.peek() || session.fixedHeight !== undefined) {
          return;
        }

        const requiredRows = Math.max(
          1,
          Math.ceil(this.getSessionContentHeight(session) / cell.height),
        );

        if (requiredRows > session.reservedRows) {
          rowsAdded += requiredRows - session.reservedRows;
          session.reservedRows = requiredRows;
        }

        this.syncSessionHostHeight(session, cell.height);
      });
    }

    this.sessions.forEach((session) => {
      const anchorRow = session.anchorMarker.line;
      if (anchorRow < 0) {
        session.host.hidden = true;
        return;
      }

      const viewportTop = (anchorRow - viewportY) * cell.height;
      session.host.style.transform = `translate3d(0, ${viewportTop}px, 0)`;
    });

    if (rowsAdded > 0) {
      this.terminal.write("\r\n".repeat(rowsAdded));
      this.scheduleInlineLayoutUpdate();
    }
  }

  private async reserveSessionRows(session: ModelSessionInstance) {
    if (!this.terminal || !this.isVisible.peek()) {
      return;
    }

    const cell = getTerminalCellMetrics(this.terminal);
    const requiredRows = Math.max(
      1,
      Math.ceil(this.getSessionContentHeight(session) / cell.height),
    );
    const rowsToAdd = requiredRows - session.reservedRows;

    if (rowsToAdd <= 0) {
      return;
    }

    session.reservedRows = requiredRows;
    this.syncSessionHostHeight(session, cell.height);
    await writeTerminalAsync(this.terminal, "\r\n".repeat(rowsToAdd));
    this.scheduleInlineLayoutUpdate();
  }

  private getSessionContentHeight(session: ModelSessionInstance) {
    if (session.fixedHeight !== undefined) {
      return session.fixedHeight;
    }

    return (
      session.host.firstElementChild?.scrollHeight ?? session.host.scrollHeight
    );
  }

  private lockSessionHeight(session: ModelSessionInstance) {
    const card = session.host.firstElementChild as HTMLElement | null;
    const height = card?.offsetHeight || card?.getBoundingClientRect().height || 0;
    if (!card || height <= 0) {
      return;
    }

    session.fixedHeight = height;
    card.style.height = `${height}px`;
  }

  private syncSessionHostHeight(
    session: ModelSessionInstance,
    cellHeight: number,
  ) {
    session.host.style.height = `${session.reservedRows * cellHeight}px`;
  }

  private parseTokens(
    text: string,
    requestId: string,
    conversationId: string,
  ): ModelSessionToken[] {
    const tokens: ModelSessionToken[] = [];
    let cursor = 0;

    while (cursor < text.length) {
      const nextBlock = this.findNextBlock(text, cursor);
      if (!nextBlock) {
        const markdown = text.slice(cursor);
        if (markdown) {
          tokens.push({ type: "markdown", content: markdown });
        }
        break;
      }

      if (nextBlock.index > cursor) {
        const markdown = text.slice(cursor, nextBlock.index);
        if (markdown) {
          tokens.push({ type: "markdown", content: markdown });
        }
      }

      nextBlock.token.requestId = requestId;
      nextBlock.token.conversationId = conversationId;
      tokens.push(nextBlock.token);
      cursor = nextBlock.endIndex;
    }

    return tokens;
  }

  private findNextBlock(
    text: string,
    startIndex: number,
  ): BlockParserResult | null {
    let nearest: BlockParserResult | null = null;

    for (const parser of COMMAND_PARSERS) {
      const parserStart = parser.findStart(text, startIndex);
      if (parserStart === -1) {
        continue;
      }

      const result = parser.parse(text, parserStart);
      if (!result) {
        continue;
      }

      if (!nearest || result.index < nearest.index) {
        nearest = result;
      }
    }

    return nearest;
  }

  private startEditCommand(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
    editorHeight?: number,
  ) {
    const state = this.getCommandState(session, token);
    if (
      state.status === "executing" ||
      state.status === "executed" ||
      state.status === "rejected"
    ) {
      return;
    }

    this.setCommandState(session, token.blockId, {
      ...state,
      status: "editing",
      draftCommand: state.command,
      editorHeight,
      error: undefined,
      errorMessageKey: undefined,
    });
  }

  private updateDraftCommand(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
    command: string,
  ) {
    const state = this.getCommandState(session, token);
    if (state.status !== "editing" || state.isGenerating) {
      return;
    }

    this.setCommandState(session, token.blockId, {
      ...state,
      draftCommand: command,
    });
  }

  private async saveCommandEdit(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
  ) {
    const state = this.getCommandState(session, token);
    if (state.status !== "editing" || state.isGenerating) {
      return;
    }

    const payload = this.createPayload(token, state.draftCommand);
    try {
      const nextCommand = this.options.onModifyCommand
        ? await this.options.onModifyCommand(payload)
        : undefined;
      const command = nextCommand ?? state.draftCommand;
      this.setCommandState(session, token.blockId, {
        ...state,
        status: "pending",
        command,
        draftCommand: command,
        error: undefined,
        errorMessageKey: undefined,
      });
    } catch (error) {
      this.setCommandState(session, token.blockId, {
        ...state,
        status: "error",
        ...this.getErrorMessageState(error, "modifyFailed"),
      });
    }
  }

  private cancelCommandEdit(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
  ) {
    const state = this.getCommandState(session, token);
    this.setCommandState(session, token.blockId, {
      ...state,
      status: "pending",
      draftCommand: state.command,
      error: undefined,
      errorMessageKey: undefined,
    });
  }

  private async generateCommand(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
  ) {
    const state = this.getCommandState(session, token);
    if (state.status !== "editing" || state.isGenerating) {
      return;
    }

    if (!state.draftCommand.trim()) {
      this.setCommandState(session, token.blockId, {
        ...state,
        error: undefined,
        errorMessageKey: "emptyGenerateCommand",
      });
      return;
    }

    this.setCommandState(session, token.blockId, {
      ...state,
      isGenerating: true,
      error: undefined,
      errorMessageKey: undefined,
    });

    try {
      const nextCommand = await this.options.onGenerateCommand?.(
        this.createPayload(token, state.draftCommand),
      );
      const generatedCommand = nextCommand?.trim();
      if (!generatedCommand) {
        this.setCommandState(session, token.blockId, {
          ...this.getCommandState(session, token),
          isGenerating: false,
          error: undefined,
          errorMessageKey: "modelReturnedEmptyCommand",
        });
        return;
      }

      const latestState = this.getCommandState(session, token);
      if (latestState.status !== "editing" || !latestState.isGenerating) {
        return;
      }

      this.setCommandState(session, token.blockId, {
        ...latestState,
        draftCommand: generatedCommand,
        isGenerating: false,
        error: undefined,
        errorMessageKey: undefined,
      });
    } catch (error) {
      const latestState = this.getCommandState(session, token);
      if (latestState.status !== "editing") {
        return;
      }

      this.setCommandState(session, token.blockId, {
        ...latestState,
        isGenerating: false,
        ...this.getErrorMessageState(error, "generateFailed"),
      });
    }
  }

  private async executeCommand(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
  ) {
    const state = this.getCommandState(session, token);
    if (
      state.status === "executing" ||
      state.status === "rejected" ||
      state.status === "executed" ||
      state.isGenerating
    ) {
      return;
    }

    const command = state.command;
    const payload = this.createPayload(token, command);

    this.setCommandState(session, token.blockId, {
      ...state,
      status: "executing",
      error: undefined,
      errorMessageKey: undefined,
    });

    try {
      if (this.options.onExecuteCommand) {
        await this.options.onExecuteCommand(payload);
      }
      this.setCommandState(session, token.blockId, {
        ...state,
        status: "executed",
        command,
        draftCommand: command,
        output: undefined,
        error: undefined,
        errorMessageKey: undefined,
      });
    } catch (error) {
      this.setCommandState(session, token.blockId, {
        ...state,
        status: "error",
        ...this.getErrorMessageState(error, "executeFailed"),
      });
    }
  }

  private async rejectCommand(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
  ) {
    const state = this.getCommandState(session, token);
    if (
      state.status === "executing" ||
      state.status === "executed" ||
      state.isGenerating
    ) {
      return;
    }

    const payload = this.createPayload(token, state.command);
    try {
      if (this.options.onRejectCommand) {
        await this.options.onRejectCommand(payload);
      }
      this.setCommandState(session, token.blockId, {
        ...state,
        status: "rejected",
        error: undefined,
        errorMessageKey: undefined,
      });
    } catch (error) {
      this.setCommandState(session, token.blockId, {
        ...state,
        status: "error",
        ...this.getErrorMessageState(error, "rejectFailed"),
      });
    }
  }

  private getErrorMessageState(error: unknown, fallbackKey: string) {
    return error instanceof Error
      ? { error: error.message, errorMessageKey: undefined }
      : { error: undefined, errorMessageKey: fallbackKey };
  }

  private createPayload(
    token: ModelSessionCommandToken,
    command: string,
  ): ModelSessionCommandActionPayload {
    return {
      requestId: token.requestId,
      conversationId: token.conversationId,
      blockId: token.blockId,
      command,
    };
  }

  private getCommandState(
    session: ModelSessionInstance,
    token: ModelSessionCommandToken,
  ) {
    const existing = session.commandStates.peek().get(token.blockId);
    if (existing) {
      return existing;
    }

    return {
      status: "pending",
      command: token.command,
      draftCommand: token.command,
    } satisfies ModelSessionCommandCardState;
  }

  private setCommandState(
    session: ModelSessionInstance,
    blockId: string,
    state: ModelSessionCommandCardState,
  ) {
    const nextStates = new Map(session.commandStates.peek());
    nextStates.set(blockId, state);
    session.commandStates.value = nextStates;
  }
}

function createSessionKey(requestId: string, conversationId: string) {
  return `${encodeURIComponent(conversationId)}:${encodeURIComponent(
    requestId,
  )}`;
}

function normalizeSsePayload(payload: unknown): ParsedSsePayload | null {
  if (!isRecord(payload)) {
    return null;
  }

  if (isParsedSsePayload(payload)) {
    return payload;
  }

  const data = payload.data;
  if (isParsedSsePayload(data)) {
    return data;
  }

  return null;
}

function isParsedSsePayload(payload: unknown): payload is ParsedSsePayload {
  return (
    isRecord(payload) &&
    typeof payload.requestId === "string" &&
    typeof payload.conversationId === "string" &&
    typeof payload.response === "string"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseExecuteCommandBlock(
  input: string,
  startIndex: number,
): BlockParserResult | null {
  const fenced = parseFencedExecuteCommandBlock(input, startIndex);
  if (fenced) {
    return fenced;
  }

  return parseXmlExecuteCommandBlock(input, startIndex);
}

function findNextExecuteCommandStart(input: string, startIndex: number) {
  const fenced = input.indexOf("```execute_command", startIndex);
  const xml = input.indexOf("<execute_command>", startIndex);

  if (fenced === -1) {
    return xml;
  }

  if (xml === -1) {
    return fenced;
  }

  return Math.min(fenced, xml);
}

function parseFencedExecuteCommandBlock(
  input: string,
  startIndex: number,
): BlockParserResult | null {
  const opening = "```execute_command";
  if (!input.startsWith(opening, startIndex)) {
    return null;
  }

  const lineBreak = input.indexOf("\n", startIndex);
  if (lineBreak === -1) {
    return null;
  }

  const contentStart = lineBreak + 1;
  const closing = input.indexOf("\n```", contentStart);
  if (closing === -1) {
    return null;
  }

  const command = input.slice(contentStart, closing).trim();
  const token = createCommandToken(command, startIndex, closing + 4);

  return {
    index: startIndex,
    endIndex: closing + 4,
    token,
  };
}

function parseXmlExecuteCommandBlock(
  input: string,
  startIndex: number,
): BlockParserResult | null {
  const opening = "<execute_command>";
  const closing = "</execute_command>";
  if (!input.startsWith(opening, startIndex)) {
    return null;
  }

  const bodyStart = startIndex + opening.length;
  const closeIndex = input.indexOf(closing, bodyStart);
  if (closeIndex === -1) {
    return null;
  }

  const commandMatch = input
    .slice(bodyStart, closeIndex)
    .match(/<command>([\s\S]*?)<\/command>/);
  const command = commandMatch?.[1]?.trim() ?? "";

  const token = createCommandToken(
    command,
    startIndex,
    closeIndex + closing.length,
  );

  return {
    index: startIndex,
    endIndex: closeIndex + closing.length,
    token,
  };
}

function createCommandToken(
  command: string,
  startIndex: number,
  endIndex: number,
): ModelSessionCommandToken {
  const blockId = `cmd-${startIndex}-${endIndex}`;

  return {
    type: "execute_command",
    blockId,
    requestId: "",
    conversationId: "",
    command,
  };
}
