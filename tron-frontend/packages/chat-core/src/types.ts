export type ChatSessionStatus = "ACTIVE" | "ARCHIVED";
export type ChatMessageRole = "USER" | "ASSISTANT";
export type ChatMessageChannel = "MAIN_AGENT" | "TASK_AGENT";

export interface ChatSession {
  id: number;
  sessionCode: string;
  workspaceCode: string;
  userCode?: string | null;
  appCode?: string | null;
  taskCode?: string | null;
  agentCode?: string | null;
  systemAgentSessionId?: string | null;
  title: string;
  status: ChatSessionStatus;
  lastRunCode?: string | null;
  lastRunStatus?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ChatSessionGroup {
  taskCode?: string | null;
  taskTitle: string;
  sessions: ChatSession[];
}

export interface CreateChatSessionInput {
  workspaceCode: string;
  title?: string | null;
  appCode?: string | null;
  taskCode?: string | null;
  agentCode?: string | null;
}

export interface UpdateChatSessionInput {
  title: string;
}

export interface ChatMessage {
  id: number;
  messageCode: string;
  sessionCode: string;
  runCode?: string | null;
  role: ChatMessageRole;
  channel: ChatMessageChannel;
  actor: string;
  content: string;
  inputTokens?: number | null;
  outputTokens?: number | null;
  totalTokens?: number | null;
  createdAt: string;
}

export interface ChatTokenUsage {
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

export interface ChatRunSummary {
  runCode: string;
  conversationCode?: string | null;
  workspaceCode: string;
  status: "COMPLETED";
  finalOutput: string;
  stepCount: number;
  tokenUsage?: ChatTokenUsage | null;
  systemAgentSessionId?: string | null;
}

export interface ChatSendResult {
  session: ChatSession;
  run: ChatRunSummary;
  messages: ChatMessage[];
}

export interface SendChatMessageInput {
  content: string;
  workspaceCode: string;
  appCode?: string | null;
  taskCode?: string | null;
  agentCode?: string | null;
  modelCode?: string | null;
  systemModelId?: string | null;
}

export interface ChatAguiEvent {
  id: string;
  type: string;
  eventName: string;
  runCode?: string | null;
  streamKey: string;
  occurredAt: string;
  outcome: string;
  metadata: Record<string, string>;
}

export type ChatAguiStreamEventType =
  | "RUN_STARTED"
  | "RUN_FINISHED"
  | "RUN_ERROR"
  | "TEXT_MESSAGE_START"
  | "TEXT_MESSAGE_CONTENT"
  | "TEXT_MESSAGE_END"
  | "REASONING_START"
  | "REASONING_MESSAGE_START"
  | "REASONING_MESSAGE_CONTENT"
  | "REASONING_TEXT_CONTENT"
  | "REASONING_MESSAGE_END"
  | "REASONING_END"
  | "TOOL_CALL_START"
  | "TOOL_CALL_ARGS"
  | "TOOL_CALL_END"
  | "TOOL_CALL_RESULT"
  | "ACTIVITY_DELTA"
  | string;

export interface ChatAguiStreamEvent {
  type: ChatAguiStreamEventType;
  threadId?: string;
  runId?: string;
  messageId?: string;
  role?: string;
  delta?: string | unknown[];
  content?: string;
  code?: string;
  message?: string;
  toolCallId?: string;
  toolCallName?: string;
  toolCallArgs?: unknown;
  parentMessageId?: string;
  timestamp?: number;
  result?: ChatSendResult;
  [key: string]: unknown;
}

export type TranscriptAguiEvent = ChatAguiEvent | ChatAguiStreamEvent;

export interface ChatStreamHandlers {
  onAguiEvent?: (event: ChatAguiStreamEvent) => void;
  onDelta?: (delta: string) => void;
}

export interface ChatAdapterCapabilities {
  supportsAbort: boolean;
  supportsAgui: boolean;
  supportsProtocolReplay: boolean;
  supportsStreaming: boolean;
}

export interface ChatAdapter {
  capabilities: ChatAdapterCapabilities;
  createSession: (input: CreateChatSessionInput) => Promise<ChatSession>;
  loadAguiEvents: (sessionCode: string) => Promise<ChatAguiEvent[]>;
  loadMessages: (sessionCode: string) => Promise<ChatMessage[]>;
  loadProtocolEvents?: (sessionCode: string) => Promise<ChatAguiStreamEvent[]>;
  streamMessage: (
    session: ChatSession,
    input: SendChatMessageInput,
    handlers?: ChatStreamHandlers,
    signal?: AbortSignal,
  ) => Promise<ChatSendResult>;
}

export interface ChatFloatingState {
  collapsed: boolean;
  height: number;
  open: boolean;
  width: number;
  x: number;
  y: number;
}

export interface ChatClientConfig {
  accept?: string;
  agentCode?: string;
  appCode?: string;
  appKey?: string;
  baseUrl?: string;
  defaultSessionCode?: string;
  disabledInput?: boolean;
  disabledToBottom?: boolean;
  draggable?: boolean;
  floating?: boolean;
  fullScreen?: boolean;
  fullScreenMinWidth?: number;
  inputPlaceholder?: string;
  maxFileSize?: number;
  maxLimitFile?: number;
  open?: boolean;
  openAppCode?: string;
  productPrefix?: string;
  resizable?: boolean;
  showAgui?: boolean;
  showSessionList?: boolean;
  storageKey?: string;
  taskCode?: string;
  title?: string;
  tokenPrefix?: string;
  userCode?: string;
  userId?: string;
  variables?: unknown;
  workspaceCode?: string;
}

export interface ChatEnvOptions {
  fallbackPrefix?: string;
  prefix?: string;
}

export interface ChatDebugOptions {
  debugEventTypes?: string[];
  recentTurnLimit?: number;
}
