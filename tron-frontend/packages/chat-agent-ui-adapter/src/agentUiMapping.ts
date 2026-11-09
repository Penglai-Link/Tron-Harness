import * as Ui from "@personai/agent-ui-chat-core";
import { FetchTransportError, isChatSendResult, sessionAgentTagLabel, type ChatSession, type ChatMessage, type ChatMessageAttachment, type ChatSendResult, type ChatUserInputRequest, type ChatUserInputAnswer, type ChatAguiStreamEvent, type SendChatMessageInput } from "@tron/chat-core";

/** Product fields remain host-owned; the public UI only reads its neutral contract. */
interface MappedSession extends Ui.ChatSession {
  metadata: { tron: ChatSession };
}
interface MappedInput extends Ui.SendChatMessageInput {
  metadata: { tron: SendChatMessageInput };
}

export function toUiSession(session: ChatSession): MappedSession {
  return {
    id: session.id, sessionId: session.sessionCode, userId: session.userCode,
    groupId: session.taskCode, displayName: sessionAgentTagLabel(session.agentCode),
    title: session.title, status: session.status, lastRunId: session.lastRunCode,
    lastRunStatus: session.lastRunStatus, createdAt: session.createdAt, updatedAt: session.updatedAt,
    metadata: { tron: session, agentId:session.agentCode } as MappedSession["metadata"],
  };
}

function isMappedSession(session: Ui.ChatSession): session is MappedSession {
  const host = session.metadata?.tron;
  return typeof host === "object" && host !== null
    && "sessionCode" in host && host.sessionCode === session.sessionId
    && "workspaceCode" in host && (typeof host.workspaceCode === "string" || host.workspaceCode === null);
}

export function fromUiSession(session: Ui.ChatSession): ChatSession {
  if (!isMappedSession(session)) throw new Error("Chat session is missing its Tron host context");
  return {
    ...session.metadata.tron, title: session.title, status: session.status,
    lastRunCode: session.lastRunId, lastRunStatus: session.lastRunStatus,
    updatedAt: session.updatedAt,
  };
}

export function toUiAttachment(value: ChatMessageAttachment): Ui.ChatMessageAttachment {
  const { resourceCode, ...rest } = value;
  return { ...rest, resourceId: resourceCode };
}
export function fromUiAttachment(value: Ui.ChatMessageAttachment): ChatMessageAttachment {
  const { resourceId, ...rest } = value;
  return { ...rest, resourceCode: resourceId };
}
export function toUiMessage(value: ChatMessage): Ui.ChatMessage {
  const { messageCode, sessionCode, runCode, attachments, ...rest } = value;
  return { ...rest, messageId: messageCode, sessionId: sessionCode, runId: runCode,
    attachments: attachments?.map(toUiAttachment) };
}
export function fromUiAnswer(value: Ui.ChatUserInputAnswer): ChatUserInputAnswer {
  const { questionId, optionId, ...rest } = value;
  return { ...rest, questionCode: questionId, optionCode: optionId };
}
function toUiAnswer(value: ChatUserInputAnswer): Ui.ChatUserInputAnswer {
  const { questionCode, optionCode, ...rest } = value;
  return { ...rest, questionId: questionCode, optionId: optionCode };
}
export function toUiQuestion(value: ChatUserInputRequest): Ui.ChatUserInputRequest {
  const { requestCode, sessionCode, runCode, assistantMessageCode, questions, answers, ...rest } = value;
  return { ...rest, requestId: requestCode, sessionId: sessionCode, runId: runCode,
    assistantMessageId: assistantMessageCode,
    questions: questions.map(({ questionCode, options, ...question }) => ({
      ...question, questionId: questionCode,
      options: options.map(({ optionCode, ...option }) => ({ ...option, optionId: optionCode })),
    })), answers: answers == null ? answers : answers.map(toUiAnswer),
  };
}
export function toUiResult(value: ChatSendResult): Ui.ChatSendResult {
  const { runCode, conversationCode, workspaceCode, systemAgentSessionId, ...run } = value.run;
  return { session: toUiSession(value.session), messages: value.messages.map(toUiMessage),
    run: { ...run, runId: runCode, conversationId: conversationCode,
      metadata: { workspaceCode, systemAgentSessionId } },
    userInputRequest: value.userInputRequest ? toUiQuestion(value.userInputRequest) : value.userInputRequest };
}
export function toUiHistory(value: Ui.SessionHistory & { compactionDetails?: Record<string, unknown> }): Ui.SessionHistory {
  return { ...value, records: { ...value.records, steps: value.records.steps.map((step) => {
    const blocks = Ui.parseCompactionBlocks(value.compactionDetails?.[step.id]);
    return {
      ...step, name: step.name === "framework.context.compaction" ? "agent.context.compaction" : step.name,
      ...(blocks ? { compaction: { blocks } } : {}),
    };
  }) } };
}
export function toUiInput(input: SendChatMessageInput): MappedInput {
  const model = input.modelSelection;
  return { content: input.content, annotations: input.annotations ?? [], requestedSkills: input.requestedSkills ?? [], modelId: input.modelCode,
    reasoningEffort: input.reasoningEffort, clientCapabilities: input.clientCapabilities, a2uiMetadata: input.a2uiMetadata,
    attachments: input.attachments?.map(toUiAttachment),
    modelSelection: model ? {
      kind: model.kind === "Fixed" ? "fixed" : model.kind === "ProviderRoute" ? "route" : "auto",
      providerId: model.providerCode, modelId: model.modelCode, routeId: model.routeCode,
    } : model,
    metadata: { tron: input, agentId: input.agentCode } as MappedInput["metadata"] };
}
function isMappedInput(input: Ui.SendChatMessageInput): input is MappedInput {
  const host = input.metadata?.tron;
  return typeof host === "object" && host !== null && "workspaceCode" in host
    && (typeof host.workspaceCode === "string" || host.workspaceCode === null);
}
export function fromUiInput(input: Ui.SendChatMessageInput): SendChatMessageInput {
  if (!isMappedInput(input)) throw new Error("Chat input is missing its Tron host context");
  return { ...input.metadata.tron, content: input.content, annotations: input.annotations ?? [], requestedSkills: input.requestedSkills ?? [], clientCapabilities: input.clientCapabilities, a2uiMetadata: input.a2uiMetadata,
    attachments: input.attachments?.map(fromUiAttachment) };
}

const eventNames: Record<string, string> = {
  "tron.short_term_memory.context.compaction_requested": "agent.memory.context.compaction_requested",
  "tron.short_term_memory.compaction.started": "agent.memory.compaction.started",
  "tron.short_term_memory.compaction.completed": "agent.memory.compaction.completed",
  "tron.short_term_memory.compaction.failed": "agent.memory.compaction.failed",
};
type ProtocolEvent = Pick<ChatAguiStreamEvent, "type" | "outcome" | "threadId" | "runId" | "timestamp"> & {
  result?: unknown;
  [key: string]: unknown;
};

function mapProtocolResult(event: ProtocolEvent, result: unknown): unknown {
  const productCompletion = event.productCompletion === true && !isRecord(event.productCompletionError)
    && (event.type === "RUN_FINISHED" || result !== undefined);
  // Unmarked native JSON may use these keys too; only a valid product envelope is legacy.
  const validProduct = isChatSendResult(result);
  if (!productCompletion && !validProduct) return result;
  if (!validProduct) throw new Error("Invalid product completion result");
  return toUiResult(result);
}

export function toUiEvent(event: ProtocolEvent): Ui.ChatAguiStreamEvent {
  const { result, ...rest } = event;
  return { ...rest, ...(typeof event.name === "string" ? { name: eventNames[event.name] ?? event.name } : {}),
    outcome: event.outcome?.type === "interrupt" ? {
      ...event.outcome,
      interrupts: event.outcome.interrupts.map((entry) => ({
        ...entry, metadata: entry.metadata ? mapQuestionMetadata(entry.metadata) : entry.metadata,
      })),
    } : event.outcome,
    result: mapProtocolResult(event, result) };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
function mapQuestionMetadata(value: Record<string, unknown>): Record<string, unknown> {
  return { ...value, requestId: value.requestCode ?? value.requestId,
    questions: Array.isArray(value.questions) ? value.questions.map((question: unknown) => {
      if (!isRecord(question)) return question;
      return { ...question, questionId: question.questionCode ?? question.questionId,
        options: Array.isArray(question.options) ? question.options.map((option: unknown) =>
          isRecord(option) ? { ...option, optionId: option.optionCode ?? option.optionId } : option) : question.options };
    }) : value.questions };
}

export function fromUiResume(entries: Ui.ChatAguiResumeEntry[]): Ui.ChatAguiResumeEntry[] {
  return entries.map((entry) => {
    if (!isRecord(entry.payload) || !Array.isArray(entry.payload.answers)) return entry;
    return { ...entry, payload: { ...entry.payload, answers: entry.payload.answers.map((answer: unknown) => {
      if (!isRecord(answer) || typeof answer.questionId !== "string") throw new Error("Invalid terminal question answer");
      const { questionId, optionId, ...rest } = answer;
      return { ...rest, questionCode: questionId, optionCode: optionId };
    }) } };
  });
}

export async function uiRequest<T>(request: Promise<T>): Promise<T> {
  try { return await request; } catch (error) {
    if (error instanceof FetchTransportError) {
      throw new Ui.ChatAdapterError(Ui.chatErrorPresentation(error.message, typeof document !== "undefined" && document.documentElement.lang.toLowerCase().startsWith("en") ? "Run failed" : "运行失败", error.code).summary, {
        code: error.code === "CHAT_RUN_ALREADY_ACTIVE" ? "RUN_ALREADY_ACTIVE" : error.code?.replace(/^CHAT_STEER_/, "STEER_"),
        details: error.data, status: error.status,
        // These composer errors are emitted before a user message is persisted.
        // Never infer safe restoration from an arbitrary HTTP 4xx alone.
        rejectedBeforeAdmission: ["101032001", "101032002", "101032003", "101032004", "101032006", "CHAT_DIRECTORY_INVALID"].includes(error.code ?? ""),
      });
    }
    throw error;
  }
}
