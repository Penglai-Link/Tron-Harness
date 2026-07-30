import type {
  ITerminalInitOnlyOptions,
  ITerminalOptions,
} from "@xterm/xterm";
import type { AgentPromptAddonOptions } from "./agentPrompt";
import type { LocalEchoAddonOptions } from "./localEcho";
import type { ModelSessionAddonOptions } from "./modelSession";

export interface TerminalOptions
  extends ITerminalOptions,
    ITerminalInitOnlyOptions {
  agentPrompt?: string | AgentPromptAddonOptions;
  localEcho?: boolean | LocalEchoAddonOptions;
  modelSession?: boolean | ModelSessionAddonOptions;
}
