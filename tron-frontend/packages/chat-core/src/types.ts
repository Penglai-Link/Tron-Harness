export type ChatSessionStatus = "ACTIVE" | "ARCHIVED";
export type ChatSessionSourceType = "MANUAL" | "TASK" | "AUTOMATION" | "TERMINAL";
export type ChatMessageRole = "USER" | "ASSISTANT";
export type ChatMessageChannel = "MAIN_AGENT" | "TASK_AGENT";
export type ChatRunningSubmitMode = "steer" | "queue";
export type ChatSubmitMode = "send" | ChatRunningSubmitMode;
export type ReasoningEffort =
  | "None"
  | "Minimal"
  | "Low"
  | "Medium"
  | "High"
  | "XHigh"
  | "Max"
  | "Ultra";

export interface ChatSession {
  id: number;
  sessionCode: string;
  workspaceCode: string;
  userCode?: string | null;
  appCode?: string | null;
  taskCode?: string | null;
  sourceType?: string | null;
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
  pendingUserInputCount?: number;
  pendingUserInputSessionCodes?: string[];
}

export interface CreateChatSessionInput {
  workspaceCode: string;
  title?: string | null;
  appCode?: string | null;
  taskCode?: string | null;
  sourceType?: ChatSessionSourceType | null;
  agentCode?: string | null;
}

export interface UpdateChatSessionInput {
  title: string;
}

export interface ChatMessage {
  id: number | string;
  messageCode: string;
  sessionCode: string;
  runCode?: string | null;
  role: ChatMessageRole;
  channel: ChatMessageChannel;
  actor: string;
  content: string;
  attachments?: ChatMessageAttachment[];
  inputTokens?: number | null;
  outputTokens?: number | null;
  totalTokens?: number | null;
  createdAt: string;
}

export type ChatMessageAttachmentKind = "file" | "image" | "inspection";

export interface ChatMessageAttachment {
  id: string;
  kind: ChatMessageAttachmentKind;
  title?: string | null;
  imageUrl?: string | null;
  resourceCode?: string | null;
  fileName?: string | null;
  contentType?: string | null;
  byteSize?: number | null;
  metadata: Record<string, unknown>;
  summary: string;
}

export interface ChatAttachmentUploadInput {
  file: File;
  kind?: ChatMessageAttachmentKind;
  metadata?: Record<string, unknown>;
  summary?: string;
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
  reasoningEffort?: ReasoningEffort | null;
  status: "COMPLETED" | "AWAITING_USER_INPUT";
  finalOutput: string;
  stepCount: number;
  tokenUsage?: ChatTokenUsage | null;
  systemAgentSessionId?: string | null;
}

export interface ChatSendResult {
  session: ChatSession;
  run: ChatRunSummary;
  messages: ChatMessage[];
  userInputRequest?: ChatUserInputRequest | null;
}

export type ChatUserInputAnswerKind = "Option" | "Other" | "Skipped";

export interface ChatUserInputOption {
  optionCode: string;
  label: string;
  description?: string | null;
}

export interface ChatUserInputQuestion {
  questionCode: string;
  question: string;
  options: ChatUserInputOption[];
}

export interface ChatUserInputAnswer {
  questionCode: string;
  answerKind: ChatUserInputAnswerKind;
  optionCode?: string | null;
  otherText?: string | null;
}

export interface ChatUserInputRequest {
  requestCode: string;
  sessionCode: string;
  runCode: string;
  assistantMessageCode?: string | null;
  intro: string;
  questions: ChatUserInputQuestion[];
  answers?: ChatUserInputAnswer[] | null;
  status: "Pending" | "Answered";
  requestedAt: number;
  answeredAt?: number | null;
}

export interface ChatUserInputAnswerInput {
  answers: ChatUserInputAnswer[];
}

export interface ChatUserInputLabels {
  ariaLabel: string;
  progress: (current: number, total: number) => string;
  back: string;
  next: string;
  continue: string;
  continuing: string;
  otherOptionLabel: string;
  otherOptionDescription: string;
  skipOptionLabel: string;
  skipOptionDescription: string;
  otherAnswerPlaceholder: string;
  otherAnswerAriaLabel: (index: number) => string;
}

export interface SendChatMessageInput {
  content: string;
  workspaceCode: string;
  appCode?: string | null;
  taskCode?: string | null;
  agentCode?: string | null;
  modelCode?: string | null;
  systemModelId?: string | null;
  reasoningEffort?: ReasoningEffort | null;
  attachments?: ChatMessageAttachment[];
  clientCapabilities?: {
    userInputRequests?: boolean;
  };
}

export interface ChatQueuedSubmission {
  id: string;
  sessionCode: string;
  input: SendChatMessageInput;
}

export interface ChatSteerContext {
  expectedRunCode: string;
}

export interface ChatSteerResult {
  sessionCode: string;
  runCode: string;
  message: ChatMessage;
}

export type ChatActiveRunState = "RUNNING" | "FINISHING";

export interface ChatActiveRun {
  runCode: string;
  runtimeAdapterKey: string;
  supportsSteer: boolean;
  state: ChatActiveRunState;
}

export interface ChatActiveRunResult {
  sessionCode: string;
  activeRun: ChatActiveRun | null;
}

export interface ChatStopContext {
  expectedRunCode: string;
}

export interface ChatStopResult {
  sessionCode: string;
  runCode: string;
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
  supportsSteer?: boolean;
  reasoningEffort?: ReasoningEffort | null;
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
  supportsActiveRun?: boolean;
  supportsAgui: boolean;
  supportsProtocolReplay: boolean;
  supportsSteer?: boolean;
  supportsStop?: boolean;
  supportsStreaming: boolean;
  supportsUserInputRequests?: boolean;
}

export interface ChatAdapter {
  capabilities: ChatAdapterCapabilities;
  createSession: (input: CreateChatSessionInput) => Promise<ChatSession>;
  deleteSession: (sessionCode: string, signal?: AbortSignal) => Promise<ChatSession>;
  downloadAttachment?: (
    session: ChatSession,
    attachment: ChatMessageAttachment,
    signal?: AbortSignal,
  ) => Promise<Blob>;
  getActiveRun?: (
    sessionCode: string,
    signal?: AbortSignal,
  ) => Promise<ChatActiveRunResult>;
  loadAguiEvents: (sessionCode: string) => Promise<ChatAguiEvent[]>;
  loadMessages: (sessionCode: string) => Promise<ChatMessage[]>;
  loadUserInputRequests?: (
    sessionCode: string,
    signal?: AbortSignal,
  ) => Promise<ChatUserInputRequest[]>;
  answerUserInputRequestStream?: (
    session: ChatSession,
    requestCode: string,
    input: ChatUserInputAnswerInput,
    handlers?: ChatStreamHandlers,
    signal?: AbortSignal,
  ) => Promise<ChatSendResult>;
  loadProtocolEvents?: (sessionCode: string) => Promise<ChatAguiStreamEvent[]>;
  steerMessage?: (
    session: ChatSession,
    input: SendChatMessageInput,
    context: ChatSteerContext,
    signal?: AbortSignal,
  ) => Promise<ChatSteerResult>;
  stopRun?: (
    session: ChatSession,
    context: ChatStopContext,
    signal?: AbortSignal,
  ) => Promise<ChatStopResult>;
  uploadAttachment?: (
    session: ChatSession,
    input: ChatAttachmentUploadInput,
    signal?: AbortSignal,
  ) => Promise<ChatMessageAttachment>;
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
  userInputLabels?: ChatUserInputLabels;
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
