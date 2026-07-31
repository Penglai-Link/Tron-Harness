import { FitAddon } from "@xterm/addon-fit";
import { Terminal as XTerminal } from "@xterm/xterm";
import "./styles/index.scss";
import { AgentPromptAddon } from "./addons/agentPrompt";
import { LocalEchoAddon } from "./addons/localEcho";
import { ModelSessionAddon } from "./addons/modelSession";
import type { AgentPromptAddonOptions } from "./types/agentPrompt";
import type {
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
};

export class Terminal {
  terminal: XTerminal;
  modelSessionAddon?: ModelSessionAddon;
  private readonly fitAddon: FitAddon;
  private localEchoAddon?: LocalEchoAddon;
  private readonly resizeObserver: ResizeObserver;
  private fitAnimationFrame?: number;

  constructor(container: HTMLElement, params: TerminalOptions = {}) {
    const { agentPrompt, localEcho, modelSession, ...terminalOptions } = params;

    this.terminal = new XTerminal(terminalOptions);

    const terminalContainer = this.createTerminalContainer(container);
    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.open(terminalContainer);
    this.resizeObserver = new ResizeObserver(() => {
      this.scheduleFit();
    });
    this.resizeObserver.observe(terminalContainer);
    this.scheduleFit();

    const agentPromptAddon = agentPrompt
      ? new AgentPromptAddon(this.normalizeAgentPromptOptions(agentPrompt))
      : undefined;

    if (agentPromptAddon) {
      this.terminal.loadAddon(agentPromptAddon);
    }

    if (localEcho) {
      this.localEchoAddon = new LocalEchoAddon(
        this.normalizeLocalEchoOptions(localEcho),
        agentPromptAddon?.setInputState.bind(agentPromptAddon),
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
    this.resizeObserver.disconnect();
    if (this.fitAnimationFrame !== undefined) {
      window.cancelAnimationFrame(this.fitAnimationFrame);
      this.fitAnimationFrame = undefined;
    }
    this.terminal.dispose();
  }

  executeCommand(command: string) {
    if (!this.localEchoAddon) {
      return Promise.reject(new Error("终端未启用本地命令执行"));
    }

    return this.localEchoAddon.executeCommand(command);
  }

  private scheduleFit() {
    if (this.fitAnimationFrame !== undefined) {
      return;
    }

    this.fitAnimationFrame = window.requestAnimationFrame(() => {
      this.fitAnimationFrame = undefined;
      this.fitAddon.fit();
    });
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
