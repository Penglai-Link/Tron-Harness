import { deriveReasoningPreview, formatTime, truncateLabel } from "./display";
import type {
  ChatAguiStreamEvent,
  ChatDebugOptions,
  ChatMessage,
  ReasoningEffort,
  TranscriptAguiEvent,
} from "./types";

export interface ReasoningWorkCard {
  content: string;
  ended: boolean;
  endMillis: number | null;
  firstMillis: number;
  id: string;
  runCode: string;
  toolCards: ToolWorkCard[];
}

export interface ToolWorkCard {
  args: unknown;
  ended: boolean;
  firstMillis: number;
  id: string;
  logs: string[];
  parentReasoningId: string | null;
  result: string;
  runCode: string;
  started: boolean;
  toolCallId: string;
  toolName: string;
}

export interface ToolGroupWorkCard {
  firstMillis: number;
  id: string;
  runCode: string;
  toolCards: ToolWorkCard[];
}

export type AguiUserWorkCard =
  | { kind: "reasoning"; value: ReasoningWorkCard }
  | { kind: "toolGroup"; value: ToolGroupWorkCard }
  | { kind: "tool"; value: ToolWorkCard };

export interface AguiWorkRun {
  events: TranscriptAguiEvent[];
  firstMillis: number;
  runCode: string;
}

export interface TranscriptPresentation {
  debugEventsByRunCode: Map<string, TranscriptAguiEvent[]>;
  waitingForModelAnchorMessageCode: string;
  workRailsByMessageCode: Map<string, AguiWorkRun[]>;
}

export const chatScrollBottomThreshold = 96;
export const chatScrollJumpButtonThreshold = 160;
export const liveAguiStartToleranceMs = 5_000;
export const activeChatReplayPendingTurnMaxAgeMs = 30 * 60_000;

export function aguiEventType(event: TranscriptAguiEvent) {
  if (typeof event.type === "string") return event.type;
  if ("eventName" in event && typeof event.eventName === "string") return event.eventName;
  return "";
}

export function aguiEventTime(event: TranscriptAguiEvent) {
  if ("occurredAt" in event && typeof event.occurredAt === "string") return event.occurredAt;
  if ("timestamp" in event && typeof event.timestamp === "number") return new Date(event.timestamp).toISOString();
  return new Date().toISOString();
}

export function aguiEventRunCode(event: TranscriptAguiEvent) {
  if ("runId" in event && typeof event.runId === "string") return event.runId;
  if ("runCode" in event && typeof event.runCode === "string") return event.runCode;
  return "stream";
}

export function aguiEventThreadId(event: TranscriptAguiEvent) {
  return "threadId" in event && typeof event.threadId === "string" ? event.threadId : "";
}

export function aguiEventSummary(event: TranscriptAguiEvent) {
  if ("delta" in event && typeof event.delta === "string" && event.delta.trim()) return event.delta;
  if ("content" in event && typeof event.content === "string" && event.content.trim()) return event.content;
  if ("message" in event && typeof event.message === "string" && event.message.trim()) return event.message;
  if ("toolCallName" in event && typeof event.toolCallName === "string") return event.toolCallName;
  if ("metadata" in event && event.metadata && typeof event.metadata === "object") {
    const metadata = event.metadata as Record<string, string>;
    return metadata.summary ?? metadata.skillName ?? ("outcome" in event && typeof event.outcome === "string" ? event.outcome : "event received");
  }
  return "event received";
}

export function aguiEventMillis(event: TranscriptAguiEvent) {
  if ("timestamp" in event && typeof event.timestamp === "number") return event.timestamp;
  const parsed = Date.parse(aguiEventTime(event));
  return Number.isNaN(parsed) ? null : parsed;
}

const reasoningEffortValues = new Set<ReasoningEffort>([
  "None",
  "Minimal",
  "Low",
  "Medium",
  "High",
  "XHigh",
  "Max",
  "Ultra",
]);

export function aguiReasoningEffort(events: TranscriptAguiEvent[]) {
  for (const event of events) {
    const directValue = "reasoningEffort" in event ? event.reasoningEffort : undefined;
    if (typeof directValue === "string" && reasoningEffortValues.has(directValue as ReasoningEffort)) {
      return directValue as ReasoningEffort;
    }
    if ("metadata" in event && event.metadata && typeof event.metadata === "object") {
      const metadataValue = (event.metadata as Record<string, unknown>).reasoningEffort;
      if (typeof metadataValue === "string" && reasoningEffortValues.has(metadataValue as ReasoningEffort)) {
        return metadataValue as ReasoningEffort;
      }
    }
  }
  return null;
}

export function aguiEventToolCallId(event: TranscriptAguiEvent) {
  return "toolCallId" in event && typeof event.toolCallId === "string" ? event.toolCallId : "";
}

export function aguiEventToolName(event: TranscriptAguiEvent) {
  if ("toolCallName" in event && typeof event.toolCallName === "string" && event.toolCallName.trim()) return event.toolCallName;
  return aguiEventSummary(event) || "tool";
}

export function aguiEventToolArgs(event: TranscriptAguiEvent) {
  return "toolCallArgs" in event ? event.toolCallArgs : undefined;
}

function aguiEventStableKey(event: TranscriptAguiEvent) {
  const messageId = "messageId" in event && typeof event.messageId === "string" ? event.messageId : "";
  const toolCallId = "toolCallId" in event && typeof event.toolCallId === "string" ? event.toolCallId : "";
  return [
    aguiEventRunCode(event),
    aguiEventType(event),
    aguiEventTime(event),
    messageId,
    toolCallId,
    aguiEventSummary(event),
  ].join("\u0000");
}

export function mergeTranscriptAguiEvents<T extends TranscriptAguiEvent>(...sources: T[][]) {
  const seen = new Set<string>();
  const merged: T[] = [];
  sources.flat().forEach((event) => {
    const key = aguiEventStableKey(event);
    if (seen.has(key)) return;
    seen.add(key);
    merged.push(event);
  });
  return merged.sort((left, right) => (aguiEventMillis(left) ?? 0) - (aguiEventMillis(right) ?? 0));
}

function latestChatMessage(messages: ChatMessage[]) {
  return messages.length ? messages[messages.length - 1] : null;
}

function messageCreatedMillis(message: ChatMessage) {
  const parsed = Date.parse(message.createdAt);
  return Number.isNaN(parsed) ? null : parsed;
}

export function isRecentPendingUserTurn(messages: ChatMessage[], now = Date.now()) {
  const latestMessage = latestChatMessage(messages);
  if (!latestMessage || latestMessage.role !== "USER") return false;
  const createdMillis = messageCreatedMillis(latestMessage);
  if (createdMillis === null) return true;
  return now - createdMillis <= activeChatReplayPendingTurnMaxAgeMs;
}

function replayRunCodesForPendingUserTurn(
  events: TranscriptAguiEvent[],
  sessionCode: string,
  messages: ChatMessage[],
) {
  const runCodes = new Set<string>();
  const latestMessage = latestChatMessage(messages);
  if (!latestMessage || latestMessage.role !== "USER") return runCodes;
  const latestUserMillis = messageCreatedMillis(latestMessage);
  const firstEventMillisByRun = new Map<string, number>();

  events.forEach((event) => {
    const threadId = aguiEventThreadId(event);
    if (threadId && threadId !== sessionCode) return;
    const runCode = aguiEventRunCode(event);
    if (!runCode || runCode === "stream") return;
    const eventMillis = aguiEventMillis(event);
    if (eventMillis === null) return;
    const currentFirstMillis = firstEventMillisByRun.get(runCode);
    if (currentFirstMillis === undefined || eventMillis < currentFirstMillis) {
      firstEventMillisByRun.set(runCode, eventMillis);
    }
  });

  firstEventMillisByRun.forEach((firstEventMillis, runCode) => {
    if (latestUserMillis === null || firstEventMillis >= latestUserMillis - liveAguiStartToleranceMs) {
      runCodes.add(runCode);
    }
  });
  return runCodes;
}

export function openAguiProtocolRunCodes(events: TranscriptAguiEvent[], sessionCode: string) {
  const terminalByRun = new Map<string, boolean>();
  events.forEach((event) => {
    const threadId = aguiEventThreadId(event);
    if (threadId && threadId !== sessionCode) return;
    const runCode = aguiEventRunCode(event);
    if (!runCode || runCode === "stream") return;
    const type = aguiEventType(event).toUpperCase();
    if (!terminalByRun.has(runCode)) terminalByRun.set(runCode, false);
    if (type.includes("RUN_FINISHED") || type.includes("RUN_ERROR")) {
      terminalByRun.set(runCode, true);
    }
  });
  return new Set(
    Array.from(terminalByRun.entries())
      .filter(([, terminal]) => !terminal)
      .map(([runCode]) => runCode),
  );
}

function knownTranscriptRunCodes(
  messages: ChatMessage[],
  liveAnchorMessageCodes: Record<string, string>,
  replayRunCodes: Set<string>,
) {
  const runCodes = new Set<string>();
  messages.forEach((message) => {
    if (message.runCode) runCodes.add(message.runCode);
  });
  Object.keys(liveAnchorMessageCodes).forEach((runCode) => {
    if (runCode && runCode !== "stream") runCodes.add(runCode);
  });
  replayRunCodes.forEach((runCode) => runCodes.add(runCode));
  return runCodes;
}

export function filterTranscriptAguiEventsForSession(
  events: TranscriptAguiEvent[],
  sessionCode: string,
  messages: ChatMessage[],
  liveAnchorMessageCodes: Record<string, string>,
) {
  const runCodes = knownTranscriptRunCodes(
    messages,
    liveAnchorMessageCodes,
    replayRunCodesForPendingUserTurn(events, sessionCode, messages),
  );
  if (!sessionCode || !runCodes.size) return [];
  return events.filter((event) => {
    const threadId = aguiEventThreadId(event);
    if (threadId && threadId !== sessionCode) return false;
    return runCodes.has(aguiEventRunCode(event));
  });
}

function stringifyPreview(value: unknown) {
  if (value == null || value === "") return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function formatJsonBlockValue(value: unknown) {
  if (typeof value === "string") {
    try {
      return JSON.stringify(JSON.parse(value), null, 2);
    } catch {
      return value;
    }
  }
  return stringifyPreview(value);
}

export function appendReasoningDelta(content: string, delta: string) {
  if (!content) return delta;
  if (!delta) return content;
  if (/\s$/.test(content) || /^[\s.,;:!?)]/.test(delta)) return `${content}${delta}`;
  return `${content} ${delta}`;
}

const toolTargetKeys = ["command", "cmd", "file", "fileName", "filePath", "path", "target", "query", "pattern", "url", "uri"];

function valueAtKnownKey(value: unknown, depth = 0): string {
  if (depth > 3 || value == null) return "";
  if (typeof value === "string") return value;
  if (typeof value !== "object") return "";
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = valueAtKnownKey(item, depth + 1);
      if (found) return found;
    }
    return "";
  }

  const record = value as Record<string, unknown>;
  for (const key of toolTargetKeys) {
    const direct = record[key];
    if (typeof direct === "string" && direct.trim()) return direct;
  }
  for (const child of Object.values(record)) {
    const found = valueAtKnownKey(child, depth + 1);
    if (found) return found;
  }
  return "";
}

function toolActionLabel(toolName: string, args: unknown) {
  const normalizedTool = toolName.toLowerCase();
  const target = valueAtKnownKey(args).toLowerCase();
  if (normalizedTool.includes("edit") || normalizedTool.includes("write") || normalizedTool.includes("patch")) return "Edit";
  if (target.includes("/") || target.includes(".")) {
    if (!normalizedTool.includes("exec") && !normalizedTool.includes("run") && !normalizedTool.includes("shell")) return "Edit";
  }
  return "Run";
}

export function toolSummaryLabel(toolName: string, args: unknown) {
  const target = valueAtKnownKey(args);
  const title = `${toolActionLabel(toolName, args)} ${toolName}`;
  if (!target || target === toolName) return title;
  return `${title} [${truncateLabel(target, 48)}]`;
}

function activityLines(event: TranscriptAguiEvent) {
  const delta = "delta" in event ? event.delta : undefined;
  if (typeof delta === "string") return delta.trim() ? [delta] : [];
  if (!Array.isArray(delta)) return [];

  return delta
    .map((item) => {
      if (typeof item === "string") return item;
      if (item && typeof item === "object") {
        const value = (item as { value?: unknown }).value;
        return typeof value === "string" ? value : stringifyPreview(value);
      }
      return stringifyPreview(item);
    })
    .filter(Boolean);
}

function reasoningCardKey(event: TranscriptAguiEvent) {
  return `${aguiEventRunCode(event)}:reasoning`;
}

function createReasoningCard(
  cards: Map<string, ReasoningWorkCard>,
  activeCardIdsByRun: Map<string, string>,
  latestCardIdsByRun: Map<string, string>,
  sequenceByRun: Map<string, number>,
  event: TranscriptAguiEvent,
) {
  const runCode = aguiEventRunCode(event);
  const sequence = (sequenceByRun.get(runCode) ?? 0) + 1;
  sequenceByRun.set(runCode, sequence);
  const key = `${reasoningCardKey(event)}:${sequence}`;
  const card = {
    content: "",
    ended: false,
    endMillis: null,
    firstMillis: aguiEventMillis(event) ?? Date.now(),
    id: key,
    runCode,
    toolCards: [],
  };
  cards.set(key, card);
  activeCardIdsByRun.set(runCode, key);
  latestCardIdsByRun.set(runCode, key);
  return card;
}

function activeReasoningCard(
  cards: Map<string, ReasoningWorkCard>,
  activeCardIdsByRun: Map<string, string>,
  event: TranscriptAguiEvent,
) {
  const activeCardId = activeCardIdsByRun.get(aguiEventRunCode(event));
  if (!activeCardId) return null;
  return cards.get(activeCardId) ?? null;
}

function ensureReasoningCard(
  cards: Map<string, ReasoningWorkCard>,
  activeCardIdsByRun: Map<string, string>,
  latestCardIdsByRun: Map<string, string>,
  sequenceByRun: Map<string, number>,
  event: TranscriptAguiEvent,
  options?: { forceNew?: boolean },
) {
  if (!options?.forceNew) {
    const existing = activeReasoningCard(cards, activeCardIdsByRun, event);
    if (existing) return existing;
  }
  return createReasoningCard(cards, activeCardIdsByRun, latestCardIdsByRun, sequenceByRun, event);
}

function endReasoningCard(
  cards: Map<string, ReasoningWorkCard>,
  activeCardIdsByRun: Map<string, string>,
  latestCardIdsByRun: Map<string, string>,
  sequenceByRun: Map<string, number>,
  event: TranscriptAguiEvent,
) {
  const card = ensureReasoningCard(cards, activeCardIdsByRun, latestCardIdsByRun, sequenceByRun, event);
  card.ended = true;
  card.endMillis = aguiEventMillis(event) ?? card.endMillis;
  if (aguiEventType(event).toUpperCase() === "REASONING_END") {
    activeCardIdsByRun.delete(aguiEventRunCode(event));
  }
  return card;
}

function ensureToolCard(
  cards: Map<string, ToolWorkCard>,
  reasoningCards: Map<string, ReasoningWorkCard>,
  latestReasoningCardIdsByRun: Map<string, string>,
  event: TranscriptAguiEvent,
) {
  const toolCallId = aguiEventToolCallId(event);
  if (!toolCallId) return null;
  const key = `${aguiEventRunCode(event)}:${toolCallId}`;
  const existing = cards.get(key);
  if (existing) return existing;
  const toolName = aguiEventToolName(event);
  const parentReasoningId = latestReasoningCardIdsByRun.get(aguiEventRunCode(event)) ?? null;
  const card = {
    args: aguiEventToolArgs(event),
    ended: false,
    firstMillis: aguiEventMillis(event) ?? Date.now(),
    id: key,
    logs: [],
    parentReasoningId,
    result: "",
    runCode: aguiEventRunCode(event),
    started: false,
    toolCallId,
    toolName,
  };
  cards.set(key, card);
  if (parentReasoningId) {
    const reasoningCard = reasoningCards.get(parentReasoningId);
    if (reasoningCard) reasoningCard.toolCards.push(card);
  }
  return card;
}

export function buildAguiUserWorkCards(events: TranscriptAguiEvent[]): AguiUserWorkCard[] {
  const reasoningCards = new Map<string, ReasoningWorkCard>();
  const activeReasoningCardIdsByRun = new Map<string, string>();
  const latestReasoningCardIdsByRun = new Map<string, string>();
  const reasoningSequenceByRun = new Map<string, number>();
  const structuredReasoningCardIds = new Set<string>();
  const toolCards = new Map<string, ToolWorkCard>();

  events.forEach((event) => {
    const type = aguiEventType(event).toUpperCase();

    if (type === "REASONING_START") {
      ensureReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event, { forceNew: true });
      return;
    }
    if (type === "REASONING_MESSAGE_START") {
      const activeCard = activeReasoningCard(
        reasoningCards,
        activeReasoningCardIdsByRun,
        event,
      );
      ensureReasoningCard(
        reasoningCards,
        activeReasoningCardIdsByRun,
        latestReasoningCardIdsByRun,
        reasoningSequenceByRun,
        event,
        { forceNew: Boolean(activeCard?.ended || activeCard?.content.trim()) },
      );
      return;
    }
    if (type === "REASONING_MESSAGE_CONTENT") {
      const card = ensureReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event);
      const delta = "delta" in event && typeof event.delta === "string" ? event.delta : "";
      if (!structuredReasoningCardIds.has(card.id)) {
        card.content = "";
      }
      structuredReasoningCardIds.add(card.id);
      card.content = appendReasoningDelta(card.content, delta);
      return;
    }
    if (type === "REASONING_TEXT_CONTENT") {
      const card = ensureReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event);
      if (structuredReasoningCardIds.has(card.id)) return;
      const delta = "delta" in event && typeof event.delta === "string" ? event.delta : "";
      card.content = appendReasoningDelta(card.content, delta);
      return;
    }
    if (type === "REASONING_END" || type === "REASONING_MESSAGE_END") {
      endReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event);
      return;
    }

    if (type === "TOOL_CALL_START" || type === "TOOL_CALL_ARGS") {
      const card = ensureToolCard(toolCards, reasoningCards, latestReasoningCardIdsByRun, event);
      if (!card) return;
      card.started = card.started || type === "TOOL_CALL_START";
      card.toolName = aguiEventToolName(event);
      const args = aguiEventToolArgs(event);
      if (args !== undefined) card.args = args;
      return;
    }
    if (type === "ACTIVITY_DELTA") {
      const card = ensureToolCard(toolCards, reasoningCards, latestReasoningCardIdsByRun, event);
      if (!card) return;
      card.logs.push(...activityLines(event));
      return;
    }
    if (type === "TOOL_CALL_RESULT") {
      const card = ensureToolCard(toolCards, reasoningCards, latestReasoningCardIdsByRun, event);
      if (!card) return;
      card.toolName = aguiEventToolName(event);
      const result = "content" in event && typeof event.content === "string" ? event.content : "";
      card.result = result || aguiEventSummary(event);
      return;
    }
    if (type === "TOOL_CALL_END") {
      const card = ensureToolCard(toolCards, reasoningCards, latestReasoningCardIdsByRun, event);
      if (card) card.ended = true;
    }
  });

  const toolGroupCards = new Map<string, ToolGroupWorkCard>();
  Array.from(toolCards.values())
    .filter((value) => value.started)
    .forEach((toolCard) => {
      const groupKey = `${toolCard.runCode}:tool-group:${toolCard.parentReasoningId ?? "unassigned"}`;
      const existing = toolGroupCards.get(groupKey);
      if (existing) {
        existing.toolCards.push(toolCard);
        existing.firstMillis = Math.min(existing.firstMillis, toolCard.firstMillis);
        return;
      }
      toolGroupCards.set(groupKey, {
        firstMillis: toolCard.firstMillis,
        id: groupKey,
        runCode: toolCard.runCode,
        toolCards: [toolCard],
      });
    });

  return [
    ...Array.from(reasoningCards.values())
      .filter((value) => value.content.trim() || value.toolCards.length > 0 || !value.ended)
      .map((value) => ({ kind: "reasoning" as const, value })),
    ...Array.from(toolGroupCards.values())
      .map((value) => ({
        kind: value.toolCards.length === 1 && value.toolCards[0].parentReasoningId === null
          ? "tool"
          : "toolGroup",
        value: value.toolCards.length === 1 && value.toolCards[0].parentReasoningId === null
          ? value.toolCards[0]
          : value,
      } as AguiUserWorkCard)),
  ].sort((left, right) => left.value.firstMillis - right.value.firstMillis);
}

function aguiRunsWithWorkDetails(events: TranscriptAguiEvent[], limit: number) {
  const runs = new Map<string, AguiWorkRun>();
  events.forEach((event) => {
    const runCode = aguiEventRunCode(event);
    if (!runCode) return;
    const firstMillis = aguiEventMillis(event) ?? Date.now();
    const existing = runs.get(runCode);
    if (existing) {
      existing.events.push(event);
      existing.firstMillis = Math.min(existing.firstMillis, firstMillis);
      return;
    }
    runs.set(runCode, {
      events: [event],
      firstMillis,
      runCode,
    });
  });

  return Array.from(runs.values())
    .filter((run) => (
      buildAguiUserWorkCards(run.events).length > 0
      || !run.events.some((event) => {
        const type = aguiEventType(event).toUpperCase();
        return type.includes("RUN_FINISHED") || type.includes("RUN_ERROR");
      })
    ))
    .sort((left, right) => left.firstMillis - right.firstMillis)
    .slice(-limit);
}

function userMessageCodeBeforeIndex(messages: ChatMessage[], startIndex: number) {
  for (let index = startIndex; index >= 0; index -= 1) {
    if (messages[index].role === "USER") return messages[index].messageCode;
  }
  return "";
}

function anchorMessageCodeForAguiRun(
  messages: ChatMessage[],
  runCode: string,
  liveAnchorMessageCodes: Record<string, string>,
) {
  const assistantIndex = messages.findIndex((message) => (
    message.role === "ASSISTANT" && message.runCode === runCode
  ));
  if (assistantIndex > 0) {
    return userMessageCodeBeforeIndex(messages, assistantIndex - 1);
  }
  const liveAnchorMessageCode = liveAnchorMessageCodes[runCode] ?? "";
  if (liveAnchorMessageCode && messages.some((message) => message.messageCode === liveAnchorMessageCode)) {
    return liveAnchorMessageCode;
  }
  return userMessageCodeBeforeIndex(messages, messages.length - 1);
}

export function aguiWorkRailsByMessageCode(
  messages: ChatMessage[],
  events: TranscriptAguiEvent[],
  liveAnchorMessageCodes: Record<string, string>,
  recentWorkRunLimit: number,
) {
  const rails = new Map<string, AguiWorkRun[]>();
  aguiRunsWithWorkDetails(events, recentWorkRunLimit).forEach((run) => {
    const messageCode = anchorMessageCodeForAguiRun(messages, run.runCode, liveAnchorMessageCodes);
    if (!messageCode) return;
    const current = rails.get(messageCode) ?? [];
    current.push(run);
    rails.set(messageCode, current);
  });
  return rails;
}

export function buildTranscriptPresentation(args: {
  debugOptions?: ChatDebugOptions;
  liveAguiAnchorMessageCodes?: Record<string, string>;
  liveAguiEvents?: ChatAguiStreamEvent[];
  liveAguiPending?: boolean;
  messages: ChatMessage[];
  protocolEvents?: ChatAguiStreamEvent[];
  sessionCode: string;
}) {
  const liveAnchorMessageCodes = args.liveAguiAnchorMessageCodes ?? {};
  const liveEvents = args.liveAguiEvents ?? [];
  const protocolEvents = args.protocolEvents ?? [];
  const recentTurnLimit = args.debugOptions?.recentTurnLimit ?? 5;
  const debugEventTypes = args.debugOptions?.debugEventTypes ?? [];
  const transcriptAguiEvents = args.liveAguiPending && !protocolEvents.length && !liveEvents.length
    ? []
    : filterTranscriptAguiEventsForSession(
      mergeTranscriptAguiEvents(protocolEvents, liveEvents),
      args.sessionCode,
      args.messages,
      liveAnchorMessageCodes,
    );
  const waitingForModelAnchorMessageCode = (() => {
    if (!args.liveAguiPending || !args.sessionCode) return "";
    if (transcriptAguiEvents.some((event) => aguiEventType(event).toUpperCase() === "RUN_STARTED")) {
      return "";
    }
    for (let index = args.messages.length - 1; index >= 0; index -= 1) {
      const message = args.messages[index];
      if (message.sessionCode === args.sessionCode && message.role === "USER") {
        return message.messageCode;
      }
    }
    return "";
  })();
  const workRailsByMessageCode = aguiWorkRailsByMessageCode(
    args.messages,
    transcriptAguiEvents,
    liveAnchorMessageCodes,
    recentTurnLimit,
  );
  const debugEventsByRunCode = new Map<string, TranscriptAguiEvent[]>();
  if (debugEventTypes.length) {
    transcriptAguiEvents.forEach((event) => {
      const type = aguiEventType(event).toUpperCase();
      if (!debugEventTypes.includes(type)) return;
      const runCode = aguiEventRunCode(event);
      const current = debugEventsByRunCode.get(runCode) ?? [];
      current.push(event);
      debugEventsByRunCode.set(runCode, current);
    });
  }
  return {
    debugEventsByRunCode,
    waitingForModelAnchorMessageCode,
    workRailsByMessageCode,
  } satisfies TranscriptPresentation;
}

export function buildMockStreamingEvents(
  sessionCode: string,
  runCode: string,
  assistantMessageCode: string,
  assistantContent: string,
) {
  const startedAt = Date.now();
  const reasoningContent = "Reviewing session context and preparing a response.";
  const toolCallId = `${runCode}-tool-1`;
  const chunks = assistantContent.match(/.{1,18}/g) ?? [assistantContent];
  const events: ChatAguiStreamEvent[] = [
    { type: "RUN_STARTED", threadId: sessionCode, runId: runCode, timestamp: startedAt },
    { type: "REASONING_START", threadId: sessionCode, runId: runCode, messageId: `${runCode}-reasoning`, timestamp: startedAt + 1 },
    { type: "REASONING_MESSAGE_START", threadId: sessionCode, runId: runCode, messageId: `${runCode}-reasoning`, timestamp: startedAt + 2 },
    { type: "REASONING_MESSAGE_CONTENT", threadId: sessionCode, runId: runCode, messageId: `${runCode}-reasoning`, delta: reasoningContent, timestamp: startedAt + 3 },
    { type: "REASONING_MESSAGE_END", threadId: sessionCode, runId: runCode, messageId: `${runCode}-reasoning`, timestamp: startedAt + 4 },
    { type: "REASONING_END", threadId: sessionCode, runId: runCode, messageId: `${runCode}-reasoning`, timestamp: startedAt + 5 },
    { type: "TOOL_CALL_START", threadId: sessionCode, runId: runCode, toolCallId, toolCallName: "workspace.inspect", toolCallArgs: {}, timestamp: startedAt + 6 },
    { type: "TOOL_CALL_RESULT", threadId: sessionCode, runId: runCode, toolCallId, toolCallName: "workspace.inspect", content: "Workspace context loaded.", timestamp: startedAt + 7 },
    { type: "TOOL_CALL_END", threadId: sessionCode, runId: runCode, toolCallId, timestamp: startedAt + 8 },
    { type: "TEXT_MESSAGE_START", threadId: sessionCode, runId: runCode, messageId: assistantMessageCode, role: "assistant", timestamp: startedAt + 9 },
    ...chunks.map((delta, index) => ({
      type: "TEXT_MESSAGE_CONTENT" as const,
      threadId: sessionCode,
      runId: runCode,
      messageId: assistantMessageCode,
      delta,
      timestamp: startedAt + 10 + index,
    })),
    { type: "TEXT_MESSAGE_END", threadId: sessionCode, runId: runCode, messageId: assistantMessageCode, timestamp: startedAt + 11 + chunks.length },
  ];
  return events;
}

export function summarizeReasoning(card: ReasoningWorkCard) {
  return deriveReasoningPreview(card.content);
}

export function describeDebugEvent(event: TranscriptAguiEvent) {
  return `${aguiEventType(event)} · ${formatTime(aguiEventTime(event))} · ${aguiEventSummary(event)}`;
}
