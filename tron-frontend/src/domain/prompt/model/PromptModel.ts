export type PromptCategory = "SYSTEM" | "DEVELOPER" | "PERSONA" | "AGENT_ROLE" | "TOOL" | "TEMPLATE" | "ACTION";
export type PromptVariableType = "STRING" | "NUMBER" | "BOOLEAN" | "JSON";
export type PromptVariableSourceKind = "MANAGED" | "RUNTIME";
export type PromptRuntimeProvider =
  | "AGENT_VISIBLE_DOMAINS"
  | "CHAT_USER_MESSAGE"
  | "TERMINAL_EXPLAIN_COMMAND"
  | "TERMINAL_EXPLAIN_OUTPUT"
  | "TERMINAL_CURRENT_COMMAND"
  | "TERMINAL_USER_REQUEST"
  | "TASK_CONTEXT"
  | "RAG_TREE_TITLE"
  | "RAG_TREE_SOURCE_TEXT"
  | "RAG_TREE_CHILD_SUMMARIES";
export type PromptTranslationStatus = "PUBLISHED" | "DRAFT" | "UPDATE_AVAILABLE";

export interface PromptVariable {
  name: string;
  type: PromptVariableType;
  description: string;
  sourceKind: PromptVariableSourceKind;
  managedVariableKey: string | null;
  runtimeProvider: PromptRuntimeProvider | null;
}

export type PromptParameter = PromptVariable;

export interface PromptTranslation {
  locale: "en-US" | "zh-CN";
  displayName: string;
  promptBody: string;
  description: string | null;
  status: PromptTranslationStatus;
  sourceVersion: number;
  updatedAt: string;
}

export interface PromptTranslationPayload {
  locale: "en-US" | "zh-CN";
  displayName: string;
  promptBody: string;
  description: string | null;
  status: PromptTranslationStatus;
}

export interface PromptAnalyzedVariable {
  name: string;
  type: PromptVariableType;
}

export interface PromptRecord {
  promptCode: string;
  displayName: string;
  category: PromptCategory;
  promptBody: string;
  description: string | null;
  enabled: boolean;
  variables: PromptVariable[];
  translations?: PromptTranslation[];
  builtIn?: boolean;
  runtimeRequired?: boolean;
  catalogKey?: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface PromptPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

export interface PromptPageQuery {
  current: number;
  size: number;
  search?: string;
  category?: PromptCategory;
  enabled?: boolean;
}

export interface PromptCreatePayload {
  displayName: string;
  category: PromptCategory;
  promptBody: string;
  description: string;
  enabled: boolean;
  variables: PromptVariable[];
  translations?: PromptTranslationPayload[];
}

export interface PromptUpdatePayload {
  promptCode: string;
  expectedVersion: number;
  displayName: string;
  promptBody: string;
  description: string;
  enabled: boolean;
  variables: PromptVariable[];
  translations?: PromptTranslationPayload[];
}

export interface PromptDetailPayload {
  promptCode: string;
}

export interface PromptDuplicatePayload extends PromptDetailPayload {
  targetCategory: PromptCategory;
}

export interface PromptAnalyzeResult {
  variables: PromptAnalyzedVariable[];
  invalidVariables: string[];
}

export interface PromptTranslationDraftPayload {
  promptCode?: string;
  sourceLocale: "en-US" | "zh-CN";
  targetLocale: "en-US" | "zh-CN";
  displayName: string;
  promptBody: string;
  description?: string | null;
}
