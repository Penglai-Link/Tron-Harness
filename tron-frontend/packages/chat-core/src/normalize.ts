import type {
  ChatActiveRunState,
  ChatActiveRunResult,
  ChatMessage,
  ChatSendResult,
  ChatSession,
  ChatSteerResult,
} from "./types";

function toScreamingSnakeCase(value: string | null | undefined) {
  const normalized = value?.trim();
  if (!normalized) return "";
  if (/^[A-Z0-9_]+$/.test(normalized)) return normalized;
  return normalized
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[\s-]+/g, "_")
    .toUpperCase();
}

export function normalizeChatSession(session: ChatSession): ChatSession {
  return {
    ...session,
    lastRunStatus: session.lastRunStatus ? toScreamingSnakeCase(session.lastRunStatus) : session.lastRunStatus,
    sourceType: session.sourceType
      ? toScreamingSnakeCase(session.sourceType) as ChatSession["sourceType"]
      : session.sourceType,
    status: toScreamingSnakeCase(session.status) as ChatSession["status"],
  };
}

export function normalizeChatMessage(message: ChatMessage): ChatMessage {
  return {
    ...message,
    channel: toScreamingSnakeCase(message.channel) as ChatMessage["channel"],
    role: toScreamingSnakeCase(message.role) as ChatMessage["role"],
  };
}

export function normalizeChatSendResult(result: ChatSendResult): ChatSendResult {
  return {
    ...result,
    messages: result.messages.map(normalizeChatMessage),
    run: {
      ...result.run,
      status: toScreamingSnakeCase(result.run.status) as ChatSendResult["run"]["status"],
    },
    session: normalizeChatSession(result.session),
  };
}

export function normalizeChatSteerResult(result: ChatSteerResult): ChatSteerResult {
  return {
    ...result,
    message: normalizeChatMessage(result.message),
  };
}

export function normalizeChatActiveRunResult(
  result: ChatActiveRunResult,
): ChatActiveRunResult {
  return {
    ...result,
    activeRun: result.activeRun
      ? {
          ...result.activeRun,
          state: toScreamingSnakeCase(result.activeRun.state) as ChatActiveRunState,
        }
      : null,
  };
}
