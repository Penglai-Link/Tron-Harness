import type { MaybePromise } from "./common";

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
}

export interface ModelSessionAddonOptions {
  enabled?: boolean;
  visible?: boolean;
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
  commandStates: ReadonlyMap<string, ModelSessionCommandCardState>;
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
