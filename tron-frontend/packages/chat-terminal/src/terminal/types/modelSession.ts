import type { MaybePromise } from "./common";
import type { ChatTerminalI18n, ChatTerminalMessageKey } from "../i18n";
import type { AIStreamEvent } from "../../transport/types";

export type ModelSessionCommandStatus =
  | "pending"
  | "editing"
  | "executing"
  | "executed"
  | "rejected"
  | "error";

export type ModelSessionToken =
  | ModelSessionMarkdownToken
  | ModelSessionCommandToken;

export interface ModelSessionMarkdownToken {
  type: "markdown";
  content: string;
}

export interface ModelSessionCommandToken {
  type: "execute_command";
  blockId: string;
  requestId: string;
  conversationId: string;
  command: string;
}

export interface ModelSessionMessageChunk {
  requestId: string;
  conversationId: string;
  response: string;
}

export interface ModelSessionActivityEvent {
  requestId: string;
  conversationId: string;
  event: AIStreamEvent;
}

export interface ModelSessionCommandActionPayload {
  requestId: string;
  conversationId: string;
  blockId: string;
  command: string;
}

export interface ModelSessionCommandCardState {
  status: ModelSessionCommandStatus;
  command: string;
  draftCommand: string;
  isGenerating?: boolean;
  editorHeight?: number;
  output?: string;
  error?: string;
  errorMessageKey?: ChatTerminalMessageKey;
}

export interface ModelSessionAddonOptions {
  enabled?: boolean;
  visible?: boolean;
  i18n?: ChatTerminalI18n;
  onExecuteCommand?: (
    payload: ModelSessionCommandActionPayload,
  ) => MaybePromise<string | void>;
  onModifyCommand?: (
    payload: ModelSessionCommandActionPayload,
  ) => MaybePromise<string | undefined>;
  onGenerateCommand?: (
    payload: ModelSessionCommandActionPayload,
  ) => MaybePromise<string>;
  onRejectCommand?: (
    payload: ModelSessionCommandActionPayload,
  ) => MaybePromise<void>;
}

export interface ModelSessionRenderState {
  tokens: ModelSessionToken[];
  activityEvents: readonly AIStreamEvent[];
  commandStates: ReadonlyMap<string, ModelSessionCommandCardState>;
  i18n: ChatTerminalI18n;
}

export interface ModelSessionRenderActions {
  onExecuteCommand: (token: ModelSessionCommandToken) => void;
  onStartEditCommand: (
    token: ModelSessionCommandToken,
    editorHeight?: number,
  ) => void;
  onDraftCommandChange: (
    token: ModelSessionCommandToken,
    command: string,
  ) => void;
  onSaveCommandEdit: (token: ModelSessionCommandToken) => void;
  onGenerateCommand?: (token: ModelSessionCommandToken) => void;
  onCancelCommandEdit: (token: ModelSessionCommandToken) => void;
  onRejectCommand: (token: ModelSessionCommandToken) => void;
}
