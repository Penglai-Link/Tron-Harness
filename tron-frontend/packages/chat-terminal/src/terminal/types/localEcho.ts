import type { MaybePromise } from "./common";
import type { ChatTerminalI18n } from "../i18n";

export interface CommandExplainPayload {
  command: string;
  output: string;
}

export interface CommandExplainContext {
  signal: AbortSignal;
}

export type CommandExplain = (
  payload: CommandExplainPayload,
  context: CommandExplainContext,
) => Promise<string>;

export interface CommandExecutionContext {
  interactive: boolean;
}

export interface LocalEchoAddonOptions {
  enabled?: boolean;
  prompt?: string | (() => string);
  isInteractiveCommand?: (command: string) => boolean;
  onCancelCommand?: () => void;
  onBeforeCommand?: (command: string) => MaybePromise<boolean | void>;
  onCommand?: (
    command: string,
    onOutput: (output: string) => void,
    context: CommandExecutionContext,
  ) => MaybePromise<string | void>;
  onCommandInput?: (data: string) => MaybePromise<void>;
  onExplain?: CommandExplain;
  explainTitle?: string;
  i18n?: ChatTerminalI18n;
}
