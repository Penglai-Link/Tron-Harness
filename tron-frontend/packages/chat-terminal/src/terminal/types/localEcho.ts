import type { MaybePromise } from "./common";

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

export interface LocalEchoAddonOptions {
  enabled?: boolean;
  prompt?: string | (() => string);
  onBeforeCommand?: (command: string) => MaybePromise<boolean | void>;
  onCommand?: (
    command: string,
    onOutput: (output: string) => void,
  ) => MaybePromise<string | void>;
  onExplain?: CommandExplain;
  explainTitle?: string;
}
