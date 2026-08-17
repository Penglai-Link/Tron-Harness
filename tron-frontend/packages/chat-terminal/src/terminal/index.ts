import { FitAddon } from "@xterm/addon-fit";
import { Terminal as XTerminal } from "@xterm/xterm";
import "./styles/index.scss";
import { AgentPromptAddon } from "./addons/agentPrompt";
import {
  isInteractiveShellCommand,
  LocalEchoAddon,
} from "./addons/localEcho";
import { ModelSessionAddon } from "./addons/modelSession";
import type {
  ChatTerminalLocale,
  ChatTerminalMessageKey,
  ChatTerminalMessageOverrides,
  ChatTerminalMessages,
} from "./i18n";
import type { AgentPromptAddonOptions } from "./types/agentPrompt";
import type {
  CommandExecutionContext,
  CommandExplain,
  CommandExplainContext,
  CommandExplainPayload,
  LocalEchoAddonOptions,
} from "./types/localEcho";
import type {
  ModelSessionAddonOptions,
  ModelSessionCommandActionPayload,
  ModelSessionCommandCardState,
  ModelSessionCommandStatus,
  ModelSessionMessageChunk,
  ModelSessionRenderActions,
  ModelSessionRenderState,
  ModelSessionToken,
} from "./types/modelSession";
import type { TerminalOptions } from "./types/terminal";
import type { TerminalThemeTokenOverrides } from "./types/themes";
import {
  applyTerminalThemeTokens,
  clearTerminalThemeTokens,
  syncTerminalTheme,
} from "./utils/themes";
import type {
  ExplainButtonOptions,
  ExplainDialogOptions,
  ExplainDialogPosition,
  ExplainDialogState,
  ExplainDialogStatus,
} from "./types/ui";

export {
  AgentPromptAddon,
  LocalEchoAddon,
  ModelSessionAddon,
  type AgentPromptAddonOptions,
  type ChatTerminalLocale,
  type ChatTerminalMessageKey,
  type ChatTerminalMessageOverrides,
  type ChatTerminalMessages,
  type CommandExecutionContext,
  type CommandExplain,
  type CommandExplainContext,
  type CommandExplainPayload,
  type ExplainButtonOptions,
  type ExplainDialogOptions,
  type ExplainDialogPosition,
  type ExplainDialogState,
  type ExplainDialogStatus,
  type LocalEchoAddonOptions,
  type ModelSessionAddonOptions,
  type ModelSessionCommandActionPayload,
  type ModelSessionCommandCardState,
  type ModelSessionCommandStatus,
  type ModelSessionMessageChunk,
  type ModelSessionRenderActions,
  type ModelSessionRenderState,
  type ModelSessionToken,
  type TerminalOptions,
  type TerminalThemeTokenOverrides,
  isInteractiveShellCommand,
};

export class Terminal {
  terminal: XTerminal;
  modelSessionAddon?: ModelSessionAddon;
  private readonly fitAddon: FitAddon;
  private agentPromptAddon?: AgentPromptAddon;
  private localEchoAddon?: LocalEchoAddon;
  private readonly resizeObserver: ResizeObserver;
  private readonly terminalContainer: HTMLElement;
  private terminalThemeTokens: TerminalThemeTokenOverrides;
  private fitAnimationFrame?: number;
  private themeRefreshAnimationFrame?: number;
  private disposed = false;

  constructor(container: HTMLElement, params: TerminalOptions = {}) {
    const {
      agentPrompt,
      localEcho,
      modelSession,
      themeTokens = {},
      theme: initialTheme,
      ...terminalOptions
    } = params;

    this.terminal = new XTerminal({
      ...terminalOptions,
      allowProposedApi: true,
      theme: initialTheme,
    });
    this.terminalThemeTokens = {
      ...themeTokens,
      ansi: {
        ...(initialTheme ?? {}),
        ...themeTokens.ansi,
      },
    };

    this.terminalContainer = this.createTerminalContainer(container);
    applyTerminalThemeTokens(this.terminalContainer, themeTokens);
    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.open(this.terminalContainer);
    this.guardViewportSyncScrollArea();
    this.resizeObserver = new ResizeObserver(() => {
      this.scheduleFit();
    });
    // The terminal's parent is the element FitAddon measures. Observe both
    // layout boundaries so flex/sidebar changes cannot leave xterm at stale
    // columns while the inner viewport keeps its percentage size.
    this.resizeObserver.observe(container);
    this.resizeObserver.observe(this.terminalContainer);
    this.refreshTheme();
    this.scheduleThemeRefresh();
    this.scheduleFit();

    this.agentPromptAddon = agentPrompt
      ? new AgentPromptAddon(this.normalizeAgentPromptOptions(agentPrompt))
      : undefined;

    if (this.agentPromptAddon) {
      this.terminal.loadAddon(this.agentPromptAddon);
    }

    if (localEcho) {
      this.localEchoAddon = new LocalEchoAddon(
        this.normalizeLocalEchoOptions(localEcho),
        (enabled, input, interactive) =>
          this.agentPromptAddon?.setInputState(enabled, input, {
            hideCursor: !enabled && !interactive,
          }),
      );
      this.terminal.loadAddon(this.localEchoAddon);
    }

    if (modelSession) {
      this.modelSessionAddon = new ModelSessionAddon(
        this.normalizeModelSessionOptions(modelSession),
      );
      this.terminal.loadAddon(this.modelSessionAddon);
    }
  }

  dispose() {
    this.disposed = true;
    this.resizeObserver.disconnect();
    if (this.fitAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.fitAnimationFrame);
      this.fitAnimationFrame = undefined;
    }
    if (this.themeRefreshAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.themeRefreshAnimationFrame);
      this.themeRefreshAnimationFrame = undefined;
    }
    this.terminal.dispose();
    this.terminalContainer.remove();
  }

  executeCommand(command: string) {
    if (!this.localEchoAddon) {
      return Promise.reject(new Error("终端未启用本地命令执行"));
    }

    return this.localEchoAddon.executeCommand(command);
  }

  markPromptRendered() {
    this.localEchoAddon?.markPromptRendered();
  }

  setThemeTokens(tokens: TerminalThemeTokenOverrides) {
    this.terminalThemeTokens = {
      ...this.terminalThemeTokens,
      ...tokens,
      ansi: {
        ...this.terminalThemeTokens.ansi,
        ...tokens.ansi,
      },
    };
    applyTerminalThemeTokens(this.terminalContainer, tokens);
    this.refreshTheme();
  }

  resetTheme() {
    this.terminalThemeTokens = {};
    clearTerminalThemeTokens(this.terminalContainer);
    this.refreshTheme();
  }

  refreshTheme() {
    syncTerminalTheme(
      this.terminal,
      this.terminalContainer,
      this.terminalThemeTokens.ansi,
    );
  }

  refreshLocale() {
    this.agentPromptAddon?.refreshLocale();
    this.localEchoAddon?.refreshLocale();
    this.modelSessionAddon?.refreshLocale();
  }

  private scheduleThemeRefresh() {
    if (this.disposed) {
      return;
    }
    if (this.themeRefreshAnimationFrame !== undefined) {
      return;
    }

    this.themeRefreshAnimationFrame = window.requestAnimationFrame(() => {
      this.themeRefreshAnimationFrame = undefined;
      if (this.disposed) {
        return;
      }
      this.refreshTheme();
    });
  }

  private scheduleFit() {
    if (this.disposed) {
      return;
    }
    if (this.fitAnimationFrame !== undefined) {
      return;
    }

    this.fitAnimationFrame = window.requestAnimationFrame(() => {
      this.fitAnimationFrame = undefined;
      this.fitTerminalToContainer();
    });
  }

  private fitTerminalToContainer() {
    if (this.disposed || !this.terminalContainer.isConnected) {
      return;
    }

    const terminalElement = this.terminal.element;
    if (!terminalElement?.isConnected) {
      return;
    }

    const { width, height } = this.terminalContainer.getBoundingClientRect();
    if (width <= 0 || height <= 0) {
      return;
    }

    try {
      this.fitAddon.fit();
    } catch (error) {
      if (!this.isMissingRenderDimensionsError(error)) {
        throw error;
      }
    }
  }

  private isMissingRenderDimensionsError(error: unknown) {
    return (
      error instanceof TypeError &&
      typeof error.message === "string" &&
      error.message.includes("dimensions")
    );
  }

  private guardViewportSyncScrollArea() {
    const terminalWithViewport = (
      this.terminal as XTerminal & {
        viewport?: {
          syncScrollArea: (immediate?: boolean, force?: boolean) => void;
        };
        _core?: {
          viewport?: {
            syncScrollArea: (immediate?: boolean, force?: boolean) => void;
          };
        };
      }
    );
    const viewport =
      terminalWithViewport.viewport ?? terminalWithViewport._core?.viewport;

    if (!viewport) {
      return;
    }

    const syncScrollArea = viewport.syncScrollArea.bind(viewport);
    viewport.syncScrollArea = (immediate?: boolean, force?: boolean) => {
      if (this.disposed || !this.terminalContainer.isConnected) {
        return;
      }

      try {
        syncScrollArea(immediate, force);
      } catch (error) {
        if (!this.isMissingRenderDimensionsError(error)) {
          throw error;
        }
      }
    };
  }

  private createTerminalContainer(container: HTMLElement) {
    const terminalContainer = document.createElement("div");

    container.classList.add("ai-terminal-container");
    terminalContainer.classList.add("ai-terminal-viewport");
    terminalContainer.style.position = "relative";
    terminalContainer.style.width = "100%";
    terminalContainer.style.height = "100%";

    container.appendChild(terminalContainer);

    return terminalContainer;
  }

  private normalizeAgentPromptOptions(
    agentPrompt: string | AgentPromptAddonOptions,
  ): AgentPromptAddonOptions {
    return typeof agentPrompt === "string"
      ? { text: agentPrompt }
      : agentPrompt;
  }

  private normalizeLocalEchoOptions(
    localEcho: boolean | LocalEchoAddonOptions,
  ): LocalEchoAddonOptions {
    return typeof localEcho === "boolean" ? { enabled: localEcho } : localEcho;
  }

  private normalizeModelSessionOptions(
    modelSession: boolean | ModelSessionAddonOptions,
  ): ModelSessionAddonOptions {
    return typeof modelSession === "boolean"
      ? { enabled: modelSession }
      : modelSession;
  }
}
