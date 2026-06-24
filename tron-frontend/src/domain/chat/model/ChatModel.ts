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
  sourceType?: "Manual" | "Task" | "Automation" | string;
  sourceCode?: string | null;
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
  sourceType?: "Manual" | "Task" | "Automation" | string;
  sourceCode?: string | null;
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
  cachedInputTokens?: number | null;
  reasoningTokens?: number | null;
}

export interface ChatTokenEstimateSnapshot {
  estimatedInputTokens: number;
  messageTokenEstimate: number;
  toolSchemaTokenEstimate: number;
  contextWindowLimit?: number | null;
  effectiveInputBudget?: number | null;
  outputTokenReserve?: number | null;
  protocolOverheadReserve?: number | null;
  safetyMarginTokens?: number | null;
  messageCount: number;
  partCount: number;
  rebased: boolean;
  overBudget: boolean;
  checkpointCode?: string | null;
  modelCode: string;
  modelName: string;
  modelProtocol: string;
  usageSource: string;
  usageConfidence: string;
}

export interface ChatUsageAuditSnapshot {
  requestEstimate: ChatTokenEstimateSnapshot;
  providerUsage?: ChatTokenUsage | null;
  cumulativeRunUsage?: ChatTokenUsage | null;
  inputTokenDelta?: number | null;
  inputTokenRatio?: number | null;
  calibrationFactor?: number | null;
  responseId?: string | null;
  usageSource: string;
  usageConfidence: string;
}

export interface ChatRunSummary {
  runCode: string;
  conversationCode?: string | null;
  workspaceCode: string;
  status: "COMPLETED";
  finalOutput: string;
  stepCount: number;
  tokenUsage?: ChatTokenUsage | null;
  latestRequestEstimate?: ChatTokenEstimateSnapshot | null;
  latestUsageAudit?: ChatUsageAuditSnapshot | null;
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

export type ChatSessionTodoStatus = "pending" | "in_progress" | "completed" | "cancelled";
export type ChatSessionTodoPriority = "high" | "medium" | "low";

export interface ChatSessionTodo {
  id: number;
  todoCode: string;
  sessionCode: string;
  workspaceCode: string;
  position: number;
  content: string;
  status: ChatSessionTodoStatus;
  priority: ChatSessionTodoPriority;
  createdByRunCode?: string | null;
  updatedByRunCode?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ChatSessionCompaction {
  sessionCode: string;
  checkpointMessageCode: string;
  status: string;
  sourceStartSeq: number;
  sourceEndSeq: number;
  tailStartSeq: number;
  summaryTokenEstimate: number;
  createdAt: string;
}

export interface CreateChatCompactionInput {
  sessionCode: string;
  reason?: string | null;
  note?: string | null;
}

export type ChatSessionGoalStatus =
  | "ACTIVE"
  | "PAUSED"
  | "BLOCKED"
  | "COMPLETED"
  | "CANCELLED"
  | "CLEARED";

export interface ChatSessionGoal {
  id: number;
  goalCode: string;
  sessionCode: string;
  workspaceCode: string;
  userCode?: string | null;
  taskCode?: string | null;
  activeFamilyCode?: string | null;
  objectiveMarkdown: string;
  status: ChatSessionGoalStatus;
  runsUsed: number;
  maxRuns: number;
  lastRunCode?: string | null;
  lastInputCode?: string | null;
  lastVerdict?: string | null;
  lastReason?: string | null;
  blockedReason?: string | null;
  requiredUserInput?: string | null;
  consecutiveJudgeFailures: number;
  createdByRunCode?: string | null;
  createdAt: string;
  updatedAt: string;
  pausedAt?: string | null;
  blockedAt?: string | null;
  completedAt?: string | null;
  cancelledAt?: string | null;
  clearedAt?: string | null;
  metadataJson?: Record<string, unknown> | null;
}

export interface ChatSessionGoalCriterion {
  id: number;
  criterionCode: string;
  goalCode: string;
  sessionCode: string;
  position: number;
  text: string;
  status: "ACTIVE" | "REMOVED";
  createdAt: string;
  updatedAt: string;
  removedAt?: string | null;
}

export interface ChatSessionGoalView {
  goal: ChatSessionGoal;
  criteria: ChatSessionGoalCriterion[];
}

export interface SetChatSessionGoalInput {
  sessionCode: string;
  objective: string;
  maxRuns?: number | null;
  criteria?: string[];
}

export interface ChatSessionGoalCommand {
  sessionCode: string;
  goalCode?: string | null;
}

export interface AddChatSessionGoalCriterionInput {
  sessionCode: string;
  goalCode?: string | null;
  text: string;
}

export interface RemoveChatSessionGoalCriterionInput {
  sessionCode: string;
  goalCode: string;
  criterionCode: string;
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
