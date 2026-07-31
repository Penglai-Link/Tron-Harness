import type {
  ITerminalInitOnlyOptions,
  ITerminalOptions,
} from "@xterm/xterm";
import type { AgentPromptAddonOptions } from "./agentPrompt";
import type { LocalEchoAddonOptions } from "./localEcho";
import type { ModelSessionAddonOptions } from "./modelSession";
import type { TerminalThemeTokenOverrides } from "./themes";

export interface TerminalOptions
  extends ITerminalOptions,
    ITerminalInitOnlyOptions {
  agentPrompt?: string | AgentPromptAddonOptions;
  localEcho?: boolean | LocalEchoAddonOptions;
  modelSession?: boolean | ModelSessionAddonOptions;
  themeTokens?: TerminalThemeTokenOverrides;
}
