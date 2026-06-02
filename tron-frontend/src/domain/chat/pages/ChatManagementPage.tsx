import {
  Fragment,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  ArrowDown,
  Bot,
  BrainCircuit,
  CheckCircle2,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  CircleDot,
  Copy,
  Ellipsis,
  GitBranch,
  Hammer,
  LoaderCircle,
  MessageSquareText,
  Plus,
  Send,
  Square,
  Terminal,
  UserRound,
  Wrench,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  ConfirmDialog,
  TextArea,
  MarkdownContent,
  useWorkspaceSpaces,
  useConsoleShellConfig,
  useDismissibleLayer,
  useToast,
} from "@/shared";
import { cn } from "@/shared/lib/utils";
import {
  archiveChatSession,
  createChatSession,
  deleteChatSession,
  streamChatMessage,
  updateChatSession,
  useChatAguiEvents,
  useChatMessageListBySession,
  useChatSessionGroupList,
} from "@/domain/chat/hooks";
import { useAuthSession } from "@/domain/auth";
import { fetchWorkspaceSystemAgents } from "@/domain/agent/api";
import { listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import type { SystemAgentModelRecord, WorkspaceSystemAgentRecord } from "@/domain/agent/model";
import { fetchWorkspaceModelCatalog } from "@/domain/model/api";
import { getWorkspaceModelCatalog } from "@/domain/model/mock";
import type { ModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  shouldShowAguiEventType,
  useAguiDebugPreferences,
  type AguiDebugPreferences,
} from "@/domain/settings/model/aguiDebugPreferences";
import type {
  ChatAguiEvent,
  ChatAguiStreamEvent,
  ChatMessage,
  ChatSession,
  ChatSessionGroup,
  SendChatMessageInput,
} from "@/domain/chat/model";

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

const chatSessionModelSelectionStorageKey = "tron.chat.session.model.selection";

function readChatSessionModelSelections() {
  if (typeof window === "undefined") return {} as Record<string, string>;
  try {
    const raw = window.localStorage.getItem(chatSessionModelSelectionStorageKey);
    if (!raw) return {} as Record<string, string>;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return {} as Record<string, string>;
    const selections: Record<string, string> = {};
    Object.entries(parsed).forEach(([sessionCode, optionCode]) => {
      if (typeof optionCode !== "string" || !optionCode.trim()) return;
      selections[sessionCode] = optionCode;
    });
    return selections;
  } catch {
    return {} as Record<string, string>;
  }
}

function writeChatSessionModelSelections(selections: Record<string, string>) {
  if (typeof window === "undefined") return;
  try {
    if (!Object.keys(selections).length) {
      window.localStorage.removeItem(chatSessionModelSelectionStorageKey);
      return;
    }
    window.localStorage.setItem(chatSessionModelSelectionStorageKey, JSON.stringify(selections));
  } catch {
    // Ignore local persistence failures and keep in-memory behavior.
  }
}

function sessionAgentTagLabel(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") return "MAIN";
  return normalized.toUpperCase();
}

function sessionAgentTagTone(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") {
    return "border-emerald-200 bg-emerald-50 text-emerald-700";
  }
  if (normalized === "opencode") {
    return "border-amber-200 bg-amber-50 text-amber-700";
  }
  if (normalized === "codex") {
    return "border-sky-200 bg-sky-50 text-sky-700";
  }
  return "border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]";
}

function sessionAgentIconTone(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") return "text-emerald-600";
  if (normalized === "opencode") return "text-amber-600";
  if (normalized === "codex") return "text-sky-600";
  return "text-[var(--color-on-surface-variant)]";
}

async function copyPlainText(value: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  document.body.removeChild(textarea);
  if (!copied) throw new Error("Clipboard copy failed");
}

function firstSession(groups: ChatSessionGroup[]) {
  return groups.flatMap((group) => group.sessions)[0];
}

function findSession(groups: ChatSessionGroup[], sessionCode: string) {
  return groups
    .flatMap((group) => group.sessions)
    .find((session) => session.sessionCode === sessionCode);
}

function orderSessionGroups(groups: ChatSessionGroup[]) {
  return [...groups].sort((left, right) => {
    if (!left.taskCode && right.taskCode) return -1;
    if (left.taskCode && !right.taskCode) return 1;
    return left.taskTitle.localeCompare(right.taskTitle);
  });
}

function insertSession(groups: ChatSessionGroup[], session: ChatSession): ChatSessionGroup[] {
  const groupKey = session.taskCode ?? "unassigned";
  let inserted = false;
  const nextGroups = groups.map((group) => {
    const currentKey = group.taskCode ?? "unassigned";
    if (currentKey !== groupKey) return group;
    inserted = true;
    return {
      ...group,
      sessions: [session, ...group.sessions.filter((item) => item.sessionCode !== session.sessionCode)],
    };
  });

  if (inserted) return orderSessionGroups(nextGroups);

  return orderSessionGroups([
    {
      taskCode: session.taskCode,
      taskTitle: session.taskCode ?? "Chat",
      sessions: [session],
    },
    ...nextGroups,
  ]);
}

function replaceSession(groups: ChatSessionGroup[], session: ChatSession): ChatSessionGroup[] {
  return orderSessionGroups(groups.map((group) => ({
    ...group,
    sessions: group.sessions.map((item) => (
      item.sessionCode === session.sessionCode ? session : item
    )),
  })));
}

function removeSession(groups: ChatSessionGroup[], sessionCode: string): ChatSessionGroup[] {
  return orderSessionGroups(groups
    .map((group) => ({
      ...group,
      sessions: group.sessions.filter((session) => session.sessionCode !== sessionCode),
    }))
    .filter((group) => group.sessions.length > 0));
}

function latestEventValue(events: ChatAguiEvent[], key: string) {
  return [...events].reverse().find((event) => event.metadata[key])?.metadata[key] ?? null;
}

function contextValue(value: string | null | undefined) {
  const trimmed = value?.trim();
  return trimmed && trimmed !== "unavailable" ? trimmed : null;
}

function formatTokenCount(value: number | null | undefined) {
  if (value == null) return "unavailable";
  return new Intl.NumberFormat("en-US").format(value);
}

function tokenField(value: number | null | undefined) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function tokenFieldFromMetadata(value: string | null | undefined) {
  if (!value) return null;
  const normalized = value.replaceAll(",", "").trim();
  if (!normalized) return null;
  const parsed = Number.parseInt(normalized, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

function chatTokenUsage(messages: ChatMessage[]) {
  const assistantMessages = messages.filter((message) => message.role === "ASSISTANT");
  let hasUsage = assistantMessages.length === 0;
  let input = 0;
  let output = 0;
  let total = 0;

  assistantMessages.forEach((message) => {
    const inputTokens = tokenField(message.inputTokens);
    const outputTokens = tokenField(message.outputTokens);
    const totalTokens = tokenField(message.totalTokens);
    if (inputTokens == null && outputTokens == null && totalTokens == null) return;

    hasUsage = true;
    input += inputTokens ?? 0;
    output += outputTokens ?? 0;
    total += totalTokens ?? (inputTokens ?? 0) + (outputTokens ?? 0);
  });

  return {
    input: hasUsage ? input : null,
    output: hasUsage ? output : null,
    total: hasUsage ? total : null,
  };
}

function lastTurnTokenUsage(messages: ChatMessage[]) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role !== "ASSISTANT") continue;
    const inputTokens = tokenField(message.inputTokens);
    const outputTokens = tokenField(message.outputTokens);
    const totalTokens = tokenField(message.totalTokens);
    if (inputTokens == null && outputTokens == null && totalTokens == null) continue;
    return {
      input: inputTokens,
      output: outputTokens,
      total: totalTokens ?? (inputTokens ?? 0) + (outputTokens ?? 0),
    };
  }
  return {
    input: null,
    output: null,
    total: null,
  };
}

function contextWindowLimit(
  model: ModelRecord | null | undefined,
  selectedModel: ChatModelOption | null | undefined,
  events: ChatAguiEvent[],
) {
  const metadataWindow = tokenFieldFromMetadata(
    contextValue(latestEventValue(events, "contextWindowTokens"))
    ?? contextValue(latestEventValue(events, "context_window_tokens"))
    ?? contextValue(latestEventValue(events, "contextWindowLimit"))
    ?? contextValue(latestEventValue(events, "context_window_limit")),
  );
  return model?.contextWindowTokens ?? selectedModel?.contextWindowTokens ?? metadataWindow ?? null;
}

function formatContextUsagePercent(usedTokens: number | null, windowTokens: number | null) {
  if (usedTokens == null || windowTokens == null || windowTokens <= 0) return null;
  const percent = (usedTokens / windowTokens) * 100;
  if (percent > 0 && percent < 0.01) return "<0.01% used";
  const maximumFractionDigits = percent < 1 ? 2 : percent < 10 ? 1 : 0;
  return `${new Intl.NumberFormat("en-US", { maximumFractionDigits }).format(percent)}% used`;
}

function formatContextWindowValue(windowTokens: number | null, usedTokens: number | null) {
  const windowLabel = formatTokenCount(windowTokens);
  const usageLabel = formatContextUsagePercent(usedTokens, windowTokens);
  return usageLabel ? `${windowLabel} · ${usageLabel}` : windowLabel;
}

function insertNewlineAtCursor(textarea: HTMLTextAreaElement) {
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const nextValue = `${textarea.value.slice(0, start)}\n${textarea.value.slice(end)}`;
  textarea.value = nextValue;
  textarea.selectionStart = start + 1;
  textarea.selectionEnd = start + 1;
  return nextValue;
}

function usedSkills(events: ChatAguiEvent[]) {
  return Array.from(new Set(events.map((event) => event.metadata.skillName).filter(Boolean)));
}

function homeDirectoryFromWorktreePath(worktreePath: string | null | undefined) {
  if (!worktreePath) return null;
  const marker = "/.codex/worktrees/";
  const markerIndex = worktreePath.indexOf(marker);
  if (markerIndex <= 0) return null;
  return worktreePath.slice(0, markerIndex);
}

function systemAgentSessionPath({
  agentCode,
  sessionId,
  worktreePath,
}: {
  agentCode: string | null | undefined;
  sessionId: string | null | undefined;
  worktreePath: string | null | undefined;
}) {
  const normalizedAgentCode = contextValue(agentCode)?.toLowerCase();
  const normalizedSessionId = contextValue(sessionId);
  if (!normalizedAgentCode || !normalizedSessionId) return null;
  const homeDirectory = homeDirectoryFromWorktreePath(worktreePath) ?? "~";

  if (normalizedAgentCode === "codex") {
    return `${homeDirectory}/.codex/sessions/**/rollout-*-${normalizedSessionId}.jsonl`;
  }
  if (normalizedAgentCode === "opencode") {
    return `${homeDirectory}/.local/share/opencode/opencode.db`;
  }
  return null;
}

type TranscriptAguiEvent = ChatAguiEvent | ChatAguiStreamEvent;

const chatScrollBottomThreshold = 96;
const chatScrollJumpButtonThreshold = 160;

function aguiEventType(event: TranscriptAguiEvent) {
  if (typeof event.type === "string") return event.type;
  if ("eventName" in event && typeof event.eventName === "string") return event.eventName;
  return "";
}

function aguiEventTime(event: TranscriptAguiEvent) {
  if ("occurredAt" in event && typeof event.occurredAt === "string") return event.occurredAt;
  if ("timestamp" in event && typeof event.timestamp === "number") return new Date(event.timestamp).toISOString();
  return new Date().toISOString();
}

function aguiEventRunCode(event: TranscriptAguiEvent) {
  if ("runId" in event && typeof event.runId === "string") return event.runId;
  if ("runCode" in event && typeof event.runCode === "string") return event.runCode;
  return "stream";
}

function aguiEventSummary(event: TranscriptAguiEvent) {
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

function aguiEventTitle(type: string) {
  const normalized = type.toUpperCase();
  if (normalized.includes("REASONING")) return "Thinking";
  if (normalized.includes("TEXT_MESSAGE")) return "Text message";
  if (normalized.includes("TOOL")) return "Tool call";
  if (normalized.includes("LOG") || normalized.includes("STEP") || normalized.includes("ACTIVITY")) return "Tool log";
  if (normalized.includes("RUN")) return "Run lifecycle";
  return "AG-UI event";
}

function aguiEventIcon(type: string) {
  const normalized = type.toUpperCase();
  if (normalized.includes("REASONING")) return <BrainCircuit size={16} />;
  if (normalized.includes("TEXT_MESSAGE")) return <MessageSquareText size={16} />;
  if (normalized.includes("TOOL_CALL")) return <Hammer size={16} />;
  if (normalized.includes("LOG") || normalized.includes("STEP") || normalized.includes("ACTIVITY")) return <Terminal size={16} />;
  if (normalized.includes("RUN_ERROR")) return <CircleAlert size={16} />;
  if (normalized.includes("RUN_FINISHED")) return <CheckCircle2 size={16} />;
  if (normalized.includes("RUN_STARTED")) return <LoaderCircle size={16} />;
  if (normalized.includes("RUN")) return <CircleDot size={16} />;
  return <CircleDot size={16} />;
}

function aguiEventMillis(event: TranscriptAguiEvent) {
  if ("timestamp" in event && typeof event.timestamp === "number") return event.timestamp;
  const parsed = Date.parse(aguiEventTime(event));
  return Number.isNaN(parsed) ? null : parsed;
}

function terminalAguiEvent(events: TranscriptAguiEvent[]) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const normalized = aguiEventType(events[index]).toUpperCase();
    if (normalized.includes("RUN_FINISHED") || normalized.includes("RUN_ERROR")) return events[index];
  }
  return null;
}

function formatAguiDuration(milliseconds: number) {
  if (milliseconds < 1000) return "<1s";
  const totalSeconds = Math.max(1, Math.round(milliseconds / 1000));
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (totalMinutes < 60) return seconds ? `${totalMinutes}m ${seconds}s` : `${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes ? `${hours}h ${minutes}m` : `${hours}h`;
}

function aguiWorkDuration(events: TranscriptAguiEvent[]) {
  const times = events
    .map(aguiEventMillis)
    .filter((value): value is number => typeof value === "number");
  if (times.length < 2) return "<1s";
  return formatAguiDuration(Math.max(...times) - Math.min(...times));
}

function latestAguiRunCode(events: TranscriptAguiEvent[]) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    if (aguiEventType(events[index]).toUpperCase() === "RUN_STARTED") {
      return aguiEventRunCode(events[index]);
    }
  }
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const runCode = aguiEventRunCode(events[index]);
    if (runCode && runCode !== "stream") return runCode;
  }
  return "";
}

function eventsForLatestAguiRun(events: TranscriptAguiEvent[]) {
  const runCode = latestAguiRunCode(events);
  if (!runCode) return events;
  return events.filter((event) => aguiEventRunCode(event) === runCode);
}

function aguiEventMessageId(event: TranscriptAguiEvent) {
  return "messageId" in event && typeof event.messageId === "string" ? event.messageId : "";
}

function aguiEventToolCallId(event: TranscriptAguiEvent) {
  return "toolCallId" in event && typeof event.toolCallId === "string" ? event.toolCallId : "";
}

function aguiEventToolName(event: TranscriptAguiEvent) {
  if ("toolCallName" in event && typeof event.toolCallName === "string" && event.toolCallName.trim()) return event.toolCallName;
  return aguiEventSummary(event) || "tool";
}

function aguiEventToolArgs(event: TranscriptAguiEvent) {
  return "toolCallArgs" in event ? event.toolCallArgs : undefined;
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

function formatJsonBlockValue(value: unknown) {
  if (typeof value === "string") {
    try {
      return JSON.stringify(JSON.parse(value), null, 2);
    } catch {
      return value;
    }
  }
  return stringifyPreview(value);
}

function truncateLabel(value: string, maxLength = 64) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) return normalized;
  return `${normalized.slice(0, Math.max(0, maxLength - 3)).trimEnd()}...`;
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

function toolSummaryLabel(toolName: string, args: unknown) {
  const target = valueAtKnownKey(args) || toolName;
  return `${toolActionLabel(toolName, args)} ${truncateLabel(target, 72)}`;
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

interface ReasoningWorkCard {
  content: string;
  ended: boolean;
  endMillis: number | null;
  firstMillis: number;
  id: string;
  runCode: string;
}

interface ToolWorkCard {
  args: unknown;
  ended: boolean;
  firstMillis: number;
  id: string;
  logs: string[];
  result: string;
  runCode: string;
  started: boolean;
  toolCallId: string;
  toolName: string;
}

type AguiUserWorkCard =
  | { kind: "reasoning"; value: ReasoningWorkCard }
  | { kind: "tool"; value: ToolWorkCard };

function reasoningCardKey(event: TranscriptAguiEvent) {
  const runCode = aguiEventRunCode(event);
  return `${runCode}:reasoning`;
}

function ensureReasoningCard(cards: Map<string, ReasoningWorkCard>, event: TranscriptAguiEvent) {
  const key = reasoningCardKey(event);
  const existing = cards.get(key);
  if (existing) return existing;
  const card = {
    content: "",
    ended: false,
    endMillis: null,
    firstMillis: aguiEventMillis(event) ?? Date.now(),
    id: key,
    runCode: aguiEventRunCode(event),
  };
  cards.set(key, card);
  return card;
}

function ensureToolCard(cards: Map<string, ToolWorkCard>, event: TranscriptAguiEvent) {
  const toolCallId = aguiEventToolCallId(event);
  if (!toolCallId) return null;
  const key = `${aguiEventRunCode(event)}:${toolCallId}`;
  const existing = cards.get(key);
  if (existing) return existing;
  const toolName = aguiEventToolName(event);
  const card = {
    args: aguiEventToolArgs(event),
    ended: false,
    firstMillis: aguiEventMillis(event) ?? Date.now(),
    id: key,
    logs: [],
    result: "",
    runCode: aguiEventRunCode(event),
    started: false,
    toolCallId,
    toolName,
  };
  cards.set(key, card);
  return card;
}

function buildAguiUserWorkCards(events: TranscriptAguiEvent[]): AguiUserWorkCard[] {
  const reasoningCards = new Map<string, ReasoningWorkCard>();
  const toolCards = new Map<string, ToolWorkCard>();

  events.forEach((event) => {
    const type = aguiEventType(event).toUpperCase();

    if (type === "REASONING_START" || type === "REASONING_MESSAGE_START") {
      ensureReasoningCard(reasoningCards, event);
      return;
    }
    if (type === "REASONING_MESSAGE_CONTENT" || type === "REASONING_TEXT_CONTENT") {
      const card = ensureReasoningCard(reasoningCards, event);
      const delta = "delta" in event && typeof event.delta === "string" ? event.delta : "";
      card.content += delta;
      return;
    }
    if (type === "REASONING_END" || type === "REASONING_MESSAGE_END") {
      const card = ensureReasoningCard(reasoningCards, event);
      card.ended = true;
      card.endMillis = aguiEventMillis(event) ?? card.endMillis;
      return;
    }

    if (type === "TOOL_CALL_START" || type === "TOOL_CALL_ARGS") {
      const card = ensureToolCard(toolCards, event);
      if (!card) return;
      card.started = card.started || type === "TOOL_CALL_START";
      card.toolName = aguiEventToolName(event);
      const args = aguiEventToolArgs(event);
      if (args !== undefined) card.args = args;
      return;
    }
    if (type === "ACTIVITY_DELTA") {
      const card = ensureToolCard(toolCards, event);
      if (!card) return;
      card.logs.push(...activityLines(event));
      return;
    }
    if (type === "TOOL_CALL_RESULT") {
      const card = ensureToolCard(toolCards, event);
      if (!card) return;
      card.toolName = aguiEventToolName(event);
      const result = "content" in event && typeof event.content === "string" ? event.content : "";
      card.result = result || aguiEventSummary(event);
      return;
    }
    if (type === "TOOL_CALL_END") {
      const card = ensureToolCard(toolCards, event);
      if (card) card.ended = true;
    }
  });

  if (!reasoningCards.size) {
    const runStartEvent = events.find((event) => aguiEventType(event).toUpperCase() === "RUN_STARTED") ?? events[0];
    if (runStartEvent) {
      const runCode = aguiEventRunCode(runStartEvent) || "stream";
      const terminalEvent = terminalAguiEvent(events);
      const fallbackKey = `${runCode}:reasoning-fallback`;
      reasoningCards.set(fallbackKey, {
        content: "",
        ended: Boolean(terminalEvent),
        endMillis: terminalEvent ? aguiEventMillis(terminalEvent) : null,
        firstMillis: aguiEventMillis(runStartEvent) ?? Date.now(),
        id: fallbackKey,
        runCode,
      });
    }
  }

  return [
    ...Array.from(reasoningCards.values())
      .map((value) => ({ kind: "reasoning" as const, value })),
    ...Array.from(toolCards.values())
      .filter((value) => value.started)
      .map((value) => ({ kind: "tool" as const, value })),
  ].sort((left, right) => left.value.firstMillis - right.value.firstMillis);
}

function availableChatModels(models: ModelRecord[]) {
  return models.filter((model) => model.enabled && model.modelType === "TEXT");
}

function defaultChatModel(models: ModelRecord[]) {
  return models.find((model) => model.isDefault) ?? models[0] ?? null;
}

interface ChatModelOption {
  contextWindowTokens: number | null;
  displayName: string;
  modelCode: string | null;
  modelName: string;
  optionCode: string;
  source: "main-agent" | "system-agent";
}

interface ChatAgentOption {
  agentCode: string;
  description: string;
  models: SystemAgentModelRecord[];
  name: string;
}

const mainChatAgentOption: ChatAgentOption = {
  agentCode: "main-agent",
  description: "Platform main in-process agent runtime.",
  models: [],
  name: "Main Agent",
};

function availableChatAgents(records: WorkspaceSystemAgentRecord[]) {
  const systemAgents = records
    .filter((record) => record.available && record.enabled)
    .map((record) => ({
      agentCode: record.agentCode,
      description: record.description,
      models: record.models,
      name: record.name,
    }));
  return [mainChatAgentOption, ...systemAgents];
}

function isMainChatAgent(agentCode: string) {
  return agentCode.trim().toLowerCase() === mainChatAgentOption.agentCode;
}

function modelsForAgent(models: ModelRecord[], selectedAgent: ChatAgentOption | null): ChatModelOption[] {
  if (!selectedAgent || isMainChatAgent(selectedAgent.agentCode)) {
    return models.map((model) => ({
      contextWindowTokens: model.contextWindowTokens ?? null,
      displayName: model.displayName,
      modelCode: model.modelCode,
      modelName: model.modelName,
      optionCode: `main:${model.modelCode}`,
      source: "main-agent",
    }));
  }

  return selectedAgent.models.map((model) => ({
    contextWindowTokens: model.contextWindowTokens ?? null,
    displayName: model.displayName || model.modelId,
    modelCode: null,
    modelName: model.modelId,
    optionCode: `system:${selectedAgent.agentCode}:${model.modelId}`,
    source: "system-agent",
  }));
}

function defaultChatModelOption(options: ChatModelOption[], models: ModelRecord[]) {
  if (!options.length) return null;
  if (options[0].source === "system-agent") return options[0];
  const defaultModel = defaultChatModel(models);
  if (!defaultModel) return options[0];
  return options.find((option) => option.modelCode === defaultModel.modelCode) ?? options[0];
}

function normalizedFuzzyValue(value: string) {
  return value.trim().toLowerCase().replace(/[\s\-_/.:]+/g, "");
}

function fuzzySubsequenceMatch(query: string, target: string) {
  if (!query) return true;
  let queryIndex = 0;
  for (const char of target) {
    if (char === query[queryIndex]) queryIndex += 1;
    if (queryIndex === query.length) return true;
  }
  return false;
}

function fuzzyTextMatch(query: string, target: string) {
  const rawQuery = query.trim().toLowerCase();
  if (!rawQuery) return true;
  const rawTarget = target.toLowerCase();
  if (rawTarget.includes(rawQuery)) return true;

  const normalizedQuery = normalizedFuzzyValue(query);
  if (!normalizedQuery) return true;
  const normalizedTarget = normalizedFuzzyValue(target);
  if (normalizedTarget.includes(normalizedQuery)) return true;
  return fuzzySubsequenceMatch(normalizedQuery, normalizedTarget);
}

function fuzzyModelMatch(query: string, model: ChatModelOption) {
  return fuzzyTextMatch(query, model.displayName) || fuzzyTextMatch(query, model.modelName);
}

function chatSendErrorToastMessage(message: string) {
  const normalized = message.trim().toLowerCase();
  if (!normalized) return "Chat send failed";
  if (normalized.includes("missing field `content`") || normalized.includes("missing field \"content\"")) {
    return "Model request format was rejected by provider. Check model protocol config or switch model.";
  }
  return message;
}

function isAbortError(error: unknown) {
  if (error instanceof DOMException && error.name === "AbortError") return true;
  return error instanceof Error && error.name === "AbortError";
}

function iconButtonClass(active = false) {
  return cn(
    "inline-flex size-8 items-center justify-center rounded-md border border-transparent text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50",
    active && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
  );
}

function SessionActionMenu({
  disabled,
  onArchive,
  onDelete,
  onRename,
  session,
}: {
  disabled: boolean;
  onArchive: (session: ChatSession) => void;
  onDelete: (session: ChatSession) => void;
  onRename: (session: ChatSession) => void;
  session: ChatSession;
}) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useDismissibleLayer({
    onDismiss: () => setOpen(false),
    open,
    popupRef: menuRef,
    triggerElement: triggerRef.current,
  });

  function select(action: (session: ChatSession) => void) {
    action(session);
    setOpen(false);
  }

  return (
    <div className="relative">
      <button
        ref={triggerRef}
        className="inline-flex size-7 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
        type="button"
        aria-label={`Open actions for ${session.title}`}
        aria-expanded={open}
        disabled={disabled}
        onClick={(event) => {
          event.stopPropagation();
          setOpen((value) => !value);
        }}
      >
        <Ellipsis size={16} />
      </button>
      {open ? (
        <div
          ref={menuRef}
          className="absolute right-0 top-full z-30 mt-1 w-32 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-1 text-sm shadow-[0_12px_28px_rgba(0,0,0,0.18)]"
          role="menu"
          aria-label={`${session.title} actions`}
        >
          <button className="flex w-full rounded-md px-2.5 py-1.5 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={() => select(onRename)}>
            Rename
          </button>
          <button className="flex w-full rounded-md px-2.5 py-1.5 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={() => select(onArchive)}>
            Archive
          </button>
          <button className="flex w-full rounded-md px-2.5 py-1.5 text-left text-[var(--color-error)] hover:bg-[var(--color-error-container)]" type="button" role="menuitem" onClick={() => select(onDelete)}>
            Delete
          </button>
        </div>
      ) : null}
    </div>
  );
}

function SessionRunIndicator({
  running,
  unseen,
}: {
  running: boolean;
  unseen: boolean;
}) {
  if (running) {
    return (
      <span className="inline-flex size-5 items-center justify-center" aria-label="Session running" title="Session running">
        <LoaderCircle className="animate-spin text-[var(--color-focus-ring)]" size={15} />
      </span>
    );
  }

  if (unseen) {
    return (
      <span className="inline-flex size-5 items-center justify-center" aria-label="Unread response" title="Unread response">
        <span className="size-2 rounded-full bg-[var(--color-focus-ring)]" />
      </span>
    );
  }

  return <span className="size-5" aria-hidden="true" />;
}

function RightContextSidebar({
  events,
  messages,
  model,
  selectedModel,
  session,
}: {
  events: ChatAguiEvent[];
  messages: ChatMessage[];
  model?: ModelRecord | null;
  selectedModel?: ChatModelOption | null;
  session?: ChatSession;
}) {
  const skills = usedSkills(events);
  const appCode = contextValue(session?.appCode) ?? contextValue(latestEventValue(events, "appCode"));
  const taskCode = contextValue(session?.taskCode) ?? contextValue(latestEventValue(events, "taskCode"));
  const worktreePath = contextValue(latestEventValue(events, "localDirectory"));
  const branchName = contextValue(latestEventValue(events, "branchName"));
  const modelLabel = (
    contextValue(model?.displayName)
    ?? contextValue(model?.modelName)
    ?? contextValue(selectedModel?.displayName)
    ?? contextValue(selectedModel?.modelName)
    ?? contextValue(latestEventValue(events, "modelName"))
    ?? contextValue(latestEventValue(events, "model_name"))
    ?? "unavailable"
  );
  const usage = chatTokenUsage(messages);
  const lastTurnUsage = lastTurnTokenUsage(messages);
  const usedTokens = usage.total;
  const windowTokens = contextWindowLimit(model, selectedModel, events);
  const isSystemAgentSession = Boolean(
    session?.agentCode && !isMainChatAgent(session.agentCode),
  );
  const systemAgentSessionId = contextValue(session?.systemAgentSessionId);
  const systemSessionPath = isSystemAgentSession ? systemAgentSessionPath({
    agentCode: session?.agentCode,
    sessionId: systemAgentSessionId,
    worktreePath,
  }) : null;

  return (
    <section className="space-y-4">
      <h2 className="text-xl font-semibold">Run Context</h2>
      <div className="space-y-5 text-sm">
        <ContextBlock title="Context Window">
          <ContextRow label="model" value={modelLabel} />
          <ContextRow label="window" value={formatContextWindowValue(windowTokens, usedTokens)} />
          <ContextRow label="last turn" value={<TokenUsageCompare lastTurn={lastTurnUsage} sessionTotal={usage} />} />
          <ContextRow label="used" value={formatTokenCount(usedTokens)} />
          <ContextRow label="input" value={formatTokenCount(usage.input)} />
          <ContextRow label="output" value={formatTokenCount(usage.output)} />
        </ContextBlock>
        {(appCode || taskCode) ? (
          <ContextBlock title="App / Task">
            {appCode ? <ContextRow label="app" value={appCode} /> : null}
            {taskCode ? <ContextRow label="task" value={taskCode} /> : null}
          </ContextBlock>
        ) : null}
        {(worktreePath || branchName) ? (
          <ContextBlock title="Worktree">
            {worktreePath ? <ContextRow label="path" value={worktreePath} /> : null}
            {branchName ? <ContextRow label="branch" value={branchName} icon={<GitBranch size={14} />} /> : null}
          </ContextBlock>
        ) : null}
        {isSystemAgentSession ? (
          <ContextBlock title="System Agent Session">
            <ContextRow label="agent" value={contextValue(session?.agentCode) ?? "unavailable"} />
            <ContextRow label="session" value={systemAgentSessionId ?? "unavailable"} />
            <ContextRow label="path" value={systemSessionPath ?? "unavailable"} />
          </ContextBlock>
        ) : null}
        {skills.length ? (
          <ContextBlock title="Skills">
            <div className="flex flex-wrap gap-2">
              {skills.map((skill) => <Badge key={skill} tone="neutral">{skill}</Badge>)}
            </div>
          </ContextBlock>
        ) : null}
      </div>
    </section>
  );
}

function ContextBlock({ children, title }: { children: React.ReactNode; title: string }) {
  return (
    <section className="space-y-2 border-b border-[var(--color-outline-variant)] pb-4 last:border-b-0">
      <h3 className="text-xs font-semibold uppercase text-[var(--color-on-surface-variant)]">{title}</h3>
      {children}
    </section>
  );
}

function ContextRow({ icon, label, value }: { icon?: React.ReactNode; label: string; value: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[72px_minmax(0,1fr)] items-start gap-3">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="flex min-w-0 items-start gap-1.5 break-all font-mono text-xs text-[var(--color-on-surface)]">
        {icon}
        {value}
      </span>
    </div>
  );
}

function TokenUsageCompare({
  lastTurn,
  sessionTotal,
}: {
  lastTurn: { input: number | null; output: number | null; total: number | null };
  sessionTotal: { input: number | null; output: number | null; total: number | null };
}) {
  return (
    <span className="grid min-w-0 grid-cols-2 gap-2">
      <span className="min-w-0 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 py-1">
        <span className="block text-[10px] uppercase tracking-wide text-[var(--color-on-surface-variant)]">last turn</span>
        <span className="block">used {formatTokenCount(lastTurn.total)}</span>
        <span className="block">in {formatTokenCount(lastTurn.input)}</span>
        <span className="block">out {formatTokenCount(lastTurn.output)}</span>
      </span>
      <span className="min-w-0 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 py-1">
        <span className="block text-[10px] uppercase tracking-wide text-[var(--color-on-surface-variant)]">session total</span>
        <span className="block">used {formatTokenCount(sessionTotal.total)}</span>
        <span className="block">in {formatTokenCount(sessionTotal.input)}</span>
        <span className="block">out {formatTokenCount(sessionTotal.output)}</span>
      </span>
    </span>
  );
}

function ChatModelSelector({
  emptyLabel,
  loading,
  models,
  onSelect,
  selectedModelCode,
}: {
  emptyLabel: string;
  loading: boolean;
  models: ChatModelOption[];
  onSelect: (modelCode: string) => void;
  selectedModelCode: string;
}) {
  const [open, setOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const selectedModel = models.find((model) => model.optionCode === selectedModelCode) ?? null;
  const filteredModels = useMemo(
    () => models.filter((model) => fuzzyModelMatch(searchQuery, model)),
    [models, searchQuery],
  );
  const disabled = loading || models.length === 0;
  const selectedModelLabel = loading ? "Loading models..." : selectedModel ? selectedModel.displayName : emptyLabel;

  useDismissibleLayer({
    onDismiss: () => setOpen(false),
    open,
    popupRef: menuRef,
    triggerElement: triggerRef.current,
  });

  useEffect(() => {
    if (!open) {
      setSearchQuery("");
      return;
    }
    const frame = window.requestAnimationFrame(() => {
      searchInputRef.current?.focus();
      searchInputRef.current?.select();
    });
    return () => window.cancelAnimationFrame(frame);
  }, [open]);

  return (
    <div className="relative min-w-0">
      <button
        ref={triggerRef}
        className="inline-flex h-8 w-full min-w-0 items-center gap-2 rounded-md px-2 text-xs text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
        type="button"
        aria-label="Select chat model"
        aria-expanded={open}
        title={selectedModelLabel}
        disabled={disabled}
        onClick={() => setOpen((value) => !value)}
      >
        <Bot size={14} />
        <span className="truncate">
          {selectedModelLabel}
        </span>
        <ChevronDown size={14} />
      </button>
      {open ? (
        <div
          ref={menuRef}
          className="absolute bottom-full left-0 z-30 mb-2 max-h-72 min-w-full w-max max-w-[260px] overflow-y-auto overscroll-contain rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-1.5 text-sm shadow-[0_12px_28px_rgba(0,0,0,0.18)]"
          role="listbox"
          aria-label="Chat models"
        >
          <div className="sticky top-0 z-10 bg-[var(--color-elevated-surface)] pb-1.5">
            <input
              ref={searchInputRef}
              aria-label="Search chat models"
              className="h-8 w-full rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 text-xs text-[var(--color-on-surface)] outline-none focus:border-[var(--color-focus-ring)]"
              placeholder="Search models..."
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key !== "Enter") return;
                const firstModel = filteredModels[0];
                if (!firstModel) return;
                event.preventDefault();
                onSelect(firstModel.optionCode);
                setOpen(false);
              }}
            />
          </div>
          {filteredModels.length ? filteredModels.map((model) => (
            <button
              className={cn(
                "flex w-full items-center rounded-md px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]",
                model.optionCode === selectedModelCode && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
              )}
              key={model.optionCode}
              type="button"
              role="option"
              aria-selected={model.optionCode === selectedModelCode}
              onClick={() => {
                onSelect(model.optionCode);
                setOpen(false);
              }}
            >
              <span className="block min-w-0 truncate font-medium">{model.displayName}</span>
            </button>
          )) : (
            <p className="px-3 py-2 text-xs text-[var(--color-on-surface-variant)]">
              No models match "{searchQuery.trim()}".
            </p>
          )}
        </div>
      ) : null}
    </div>
  );
}

function ChatAgentSelector({
  locked,
  loading,
  agents,
  onSelect,
  selectedAgentCode,
}: {
  locked?: boolean;
  loading: boolean;
  agents: ChatAgentOption[];
  onSelect: (agentCode: string) => void;
  selectedAgentCode: string;
}) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const selectedAgent = agents.find((agent) => agent.agentCode === selectedAgentCode) ?? null;
  const disabled = loading || agents.length === 0 || Boolean(locked);

  useDismissibleLayer({
    onDismiss: () => setOpen(false),
    open,
    popupRef: menuRef,
    triggerElement: triggerRef.current,
  });

  return (
    <div className="relative min-w-0">
      <button
        ref={triggerRef}
        className="inline-flex h-8 w-full min-w-0 items-center gap-2 rounded-md px-2 text-xs text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
        type="button"
        aria-label="Select chat agent"
        aria-expanded={open}
        title={locked ? "Agent is locked after the first chat turn in this session" : undefined}
        disabled={disabled}
        onClick={() => setOpen((value) => !value)}
      >
        <Wrench size={14} />
        <span className="truncate">
          {loading ? "Loading agents..." : selectedAgent?.name ?? "Main Agent"}
        </span>
        <ChevronDown size={14} />
      </button>
      {open ? (
        <div
          ref={menuRef}
          className="absolute bottom-full left-0 z-30 mb-2 max-h-72 min-w-full w-max max-w-[260px] overflow-y-auto overscroll-contain rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-1.5 text-sm shadow-[0_12px_28px_rgba(0,0,0,0.18)]"
          role="listbox"
          aria-label="Chat agents"
        >
          {agents.map((agent) => (
            <button
              className={cn(
                "flex w-full flex-col items-start rounded-md px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]",
                agent.agentCode === selectedAgentCode && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
              )}
              key={agent.agentCode}
              type="button"
              role="option"
              aria-selected={agent.agentCode === selectedAgentCode}
              onClick={() => {
                onSelect(agent.agentCode);
                setOpen(false);
              }}
            >
              <span className="block min-w-0 truncate font-medium">{agent.name}</span>
              <span className="mt-0.5 block min-w-0 truncate text-[11px] text-[var(--color-on-surface-variant)]">
                {agent.agentCode}
              </span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function SessionSidebar({
  activeSessionCode,
  collapsed,
  creatingSession,
  groups,
  onCollapseToggle,
  onArchiveSession,
  onCreateSession,
  onDeleteSession,
  onRenameSession,
  onSessionSelect,
  runningSessionCode,
  sessionActionPendingCode,
  unseenSessionCodes,
}: {
  activeSessionCode: string;
  collapsed: boolean;
  creatingSession: boolean;
  groups: ChatSessionGroup[];
  onCollapseToggle: () => void;
  onArchiveSession: (session: ChatSession) => void;
  onCreateSession: () => void;
  onDeleteSession: (session: ChatSession) => void;
  onRenameSession: (session: ChatSession, title: string) => Promise<void>;
  onSessionSelect: (sessionCode: string) => void;
  runningSessionCode: string;
  sessionActionPendingCode: string;
  unseenSessionCodes: Set<string>;
}) {
  const [closedGroups, setClosedGroups] = useState<Set<string>>(() => new Set());
  const [editingSessionCode, setEditingSessionCode] = useState("");
  const [renameDraft, setRenameDraft] = useState("");
  const [collapsedTooltip, setCollapsedTooltip] = useState<{
    agentLabel: string;
    sessionTitle: string;
    top: number;
  } | null>(null);
  const collapsedTooltipTimerRef = useRef<number | null>(null);
  const sidebarRef = useRef<HTMLElement | null>(null);

  function clearCollapsedTooltipTimer() {
    if (collapsedTooltipTimerRef.current === null) return;
    window.clearTimeout(collapsedTooltipTimerRef.current);
    collapsedTooltipTimerRef.current = null;
  }

  function hideCollapsedTooltip() {
    clearCollapsedTooltipTimer();
    setCollapsedTooltip(null);
  }

  function showCollapsedTooltip(trigger: HTMLButtonElement, session: ChatSession) {
    clearCollapsedTooltipTimer();
    const triggerRect = trigger.getBoundingClientRect();
    const sidebarRect = sidebarRef.current?.getBoundingClientRect();
    const top = sidebarRect
      ? triggerRect.top - sidebarRect.top + triggerRect.height / 2
      : trigger.offsetTop + trigger.offsetHeight / 2;
    collapsedTooltipTimerRef.current = window.setTimeout(() => {
      setCollapsedTooltip({
        agentLabel: sessionAgentTagLabel(session.agentCode),
        sessionTitle: session.title,
        top,
      });
    }, 120);
  }

  function startInlineRename(session: ChatSession) {
    setEditingSessionCode(session.sessionCode);
    setRenameDraft(session.title);
  }

  function cancelInlineRename() {
    setEditingSessionCode("");
    setRenameDraft("");
  }

  async function submitInlineRename(session: ChatSession) {
    const title = renameDraft.trim();
    if (!title || title === session.title) {
      cancelInlineRename();
      return;
    }

    await onRenameSession(session, title);
    cancelInlineRename();
  }
  const collapsedSessions = groups.flatMap((group) => group.sessions);

  useEffect(() => () => clearCollapsedTooltipTimer(), []);

  useEffect(() => {
    if (!collapsed) hideCollapsedTooltip();
  }, [collapsed]);

  return (
    <aside
      ref={sidebarRef}
      className={cn(
        "relative h-full shrink-0 overflow-visible bg-[var(--color-surface-container-lowest)] shadow-[10px_0_28px_rgba(15,23,42,0.04)] transition-[width] duration-200",
        collapsed ? "w-[64px]" : "w-[286px]",
      )}
    >
      {collapsed ? (
        <div className="flex h-full flex-col overflow-y-auto px-2 py-3">
          <div className="flex items-center justify-center">
            <button
              className={iconButtonClass()}
              type="button"
              aria-label="New session"
              title="New session"
              disabled={creatingSession}
              onClick={onCreateSession}
            >
              <Plus size={17} />
            </button>
          </div>
          <div className="mx-auto mt-4 h-px w-6 bg-[rgba(15,23,42,0.06)]" />
          <div className="mt-3 flex flex-1 flex-col items-center gap-1 overflow-y-auto">
            {collapsedSessions.map((session) => (
              <button
                className={iconButtonClass(activeSessionCode === session.sessionCode)}
                key={session.sessionCode}
                type="button"
                aria-label={`Open ${session.title}`}
                onClick={() => onSessionSelect(session.sessionCode)}
                onMouseEnter={(event) => showCollapsedTooltip(event.currentTarget, session)}
                onMouseLeave={hideCollapsedTooltip}
                onFocus={(event) => showCollapsedTooltip(event.currentTarget, session)}
                onBlur={hideCollapsedTooltip}
              >
                <MessageSquareText className={sessionAgentIconTone(session.agentCode)} size={17} />
              </button>
            ))}
          </div>
          {collapsedTooltip ? (
            <div
              className="pointer-events-none absolute left-full z-40 ml-2 -translate-y-1/2 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-2 py-1.5 text-xs text-[var(--color-on-surface)] shadow-[0_8px_18px_rgba(15,23,42,0.16)]"
              style={{ top: collapsedTooltip.top }}
              role="tooltip"
            >
              <p className="text-[10px] font-semibold uppercase tracking-wide text-[var(--color-on-surface-variant)]">
                {collapsedTooltip.agentLabel}
              </p>
              <p className="max-w-[220px] truncate text-xs text-[var(--color-on-surface)]">
                {collapsedTooltip.sessionTitle}
              </p>
            </div>
          ) : null}
        </div>
      ) : (
        <>
          <div className="flex h-14 items-center justify-between px-4 shadow-[0_8px_24px_rgba(15,23,42,0.035)]">
            <div className="flex items-center gap-2">
              <MessageSquareText size={17} />
              <h2 className="text-sm font-semibold">Sessions</h2>
            </div>
            <div className="flex items-center gap-1">
              <button
                className={iconButtonClass()}
                type="button"
                aria-label="New session"
                title="New session"
                disabled={creatingSession}
                onClick={onCreateSession}
              >
                <Plus size={17} />
              </button>
            </div>
          </div>
          <div className="h-[calc(100%-3.5rem)] overflow-y-auto px-3 py-4">
            {groups.map((group) => {
              const groupKey = group.taskCode ?? "unassigned";
              const closed = closedGroups.has(groupKey);
              return (
                <section className="mb-4" key={groupKey}>
                  <button
                    className="mb-2 flex w-full items-center justify-between rounded-md px-2 py-1 text-left text-xs font-semibold uppercase text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]"
                    type="button"
                    onClick={() => {
                      setClosedGroups((current) => {
                        const next = new Set(current);
                        if (next.has(groupKey)) next.delete(groupKey);
                        else next.add(groupKey);
                        return next;
                      });
                    }}
                  >
                    <span>{group.taskTitle}</span>
                    {closed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
                  </button>
                  {!closed ? (
                    <div className="space-y-1">
                      {group.sessions.map((session) => {
                        const editing = editingSessionCode === session.sessionCode;
                        const pending = sessionActionPendingCode === session.sessionCode;

                        return (
                          <div
                            className={cn(
                              "grid w-full grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2 rounded-md px-3 py-2 text-left transition",
                              activeSessionCode === session.sessionCode
                                ? "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]"
                                : "text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]",
                            )}
                            key={session.sessionCode}
                          >
                            {editing ? (
                              <input
                                autoFocus
                                aria-label={`Rename ${session.title}`}
                                className="h-8 min-w-0 rounded-md border border-[var(--color-focus-ring)] bg-white px-2 text-sm font-medium text-[var(--color-on-surface)] outline-none"
                                disabled={pending}
                                value={renameDraft}
                                onBlur={() => {
                                  if (!pending) void submitInlineRename(session);
                                }}
                                onChange={(event) => setRenameDraft(event.target.value)}
                                onClick={(event) => event.stopPropagation()}
                                onFocus={(event) => event.currentTarget.select()}
                                onKeyDown={(event) => {
                                  if (event.key === "Enter") {
                                    event.preventDefault();
                                    void submitInlineRename(session);
                                  }
                                  if (event.key === "Escape") {
                                    event.preventDefault();
                                    cancelInlineRename();
                                  }
                                }}
                              />
                            ) : (
                              <button className="min-w-0 text-left" type="button" onClick={() => onSessionSelect(session.sessionCode)}>
                                <span className="block truncate text-sm font-medium">{session.title}</span>
                                <span className="mt-1 flex items-center gap-1.5 text-xs text-[var(--color-on-surface-variant)]">
                                  <span
                                    className={cn(
                                      "inline-flex h-4 items-center rounded border px-1 font-mono text-[10px] leading-none",
                                      sessionAgentTagTone(session.agentCode),
                                    )}
                                  >
                                    {sessionAgentTagLabel(session.agentCode)}
                                  </span>
                                  <span>{formatTime(session.updatedAt)}</span>
                                </span>
                              </button>
                            )}
                            <SessionRunIndicator
                              running={runningSessionCode === session.sessionCode}
                              unseen={unseenSessionCodes.has(session.sessionCode)}
                            />
                            <SessionActionMenu
                              disabled={pending}
                              session={session}
                              onArchive={onArchiveSession}
                              onDelete={onDeleteSession}
                              onRename={startInlineRename}
                            />
                          </div>
                        );
                      })}
                    </div>
                  ) : null}
                </section>
              );
            })}
          </div>
        </>
      )}
      <button
        className="absolute right-[-22px] top-1/2 z-10 inline-flex h-11 w-6 -translate-y-1/2 items-center justify-center rounded-l-none rounded-r-full border-0 bg-[var(--color-surface-container-lowest)] p-0 text-[var(--color-on-surface-variant)] shadow-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2"
        type="button"
        aria-label={collapsed ? "Expand sessions" : "Collapse sessions"}
        aria-pressed={collapsed}
        onClick={onCollapseToggle}
      >
        {collapsed ? <ChevronRight size={15} strokeWidth={2.4} /> : <ChevronLeft size={15} strokeWidth={2.4} />}
      </button>
    </aside>
  );
}

function ChatBubble({
  currentUserDisplayName,
  message,
}: {
  currentUserDisplayName: string;
  message: ChatMessage;
}) {
  const isUser = message.role === "USER";
  const actorName = isUser ? (currentUserDisplayName || message.actor) : message.actor;
  const [copied, setCopied] = useState(false);
  const copyResetRef = useRef<number | null>(null);

  useEffect(() => () => {
    if (copyResetRef.current !== null) window.clearTimeout(copyResetRef.current);
  }, []);

  async function handleCopyMessage() {
    if (!message.content) return;
    try {
      await copyPlainText(message.content);
      setCopied(true);
      if (copyResetRef.current !== null) window.clearTimeout(copyResetRef.current);
      copyResetRef.current = window.setTimeout(() => setCopied(false), 1400);
    } catch {
      setCopied(false);
    }
  }

  return (
    <article className={cn("max-w-[920px] rounded-md bg-white px-4 py-3 shadow-[0_8px_22px_rgba(15,23,42,0.06)]", isUser ? "ml-auto bg-[var(--color-surface-container-lowest)]" : "")}>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          {isUser ? <UserRound size={16} /> : <Bot size={16} />}
          <span className="truncate text-sm font-semibold">{actorName}</span>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {!isUser ? (
            <button
              aria-label="Copy raw markdown"
              className="inline-flex size-6 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-40"
              disabled={!message.content}
              title="Copy raw markdown"
              type="button"
              onClick={() => void handleCopyMessage()}
            >
              {copied ? <CheckCircle2 size={14} /> : <Copy size={14} />}
            </button>
          ) : null}
          <span className="text-xs text-[var(--color-on-surface-variant)]">{formatTime(message.createdAt)}</span>
        </div>
      </div>
      {isUser ? (
        <p className="whitespace-pre-wrap text-sm leading-6 text-[var(--color-on-surface)]">{message.content}</p>
      ) : (
        <MarkdownContent
          ariaLabel="Streaming assistant markdown"
          className="py-0"
          content={message.content}
          emptyMessage="Waiting for assistant output..."
        />
      )}
    </article>
  );
}

function AguiUserCardShell({
  children,
  icon,
  meta,
  summary,
  title,
}: {
  children: React.ReactNode;
  icon: React.ReactNode;
  meta?: React.ReactNode;
  summary?: React.ReactNode;
  title: React.ReactNode;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <article className="rounded-md bg-white px-3 py-2 text-sm shadow-[0_8px_22px_rgba(15,23,42,0.06)]">
      <button
        className="flex w-full items-start justify-between gap-3 text-left"
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((current) => !current)}
      >
        <div className="flex min-w-0 items-start gap-2">
          <span className="mt-0.5 inline-flex size-7 shrink-0 items-center justify-center rounded-md bg-[var(--color-surface-container-low)] text-[var(--color-on-surface-variant)]">
            {icon}
          </span>
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <span className="truncate font-semibold">{title}</span>
              {meta}
            </div>
            {summary ? <div className="mt-1 min-w-0 text-[var(--color-on-surface-variant)]">{summary}</div> : null}
          </div>
        </div>
        <span className="mt-1 shrink-0 text-[var(--color-on-surface-variant)]">
          {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        </span>
      </button>
      {expanded ? <div className="mt-3 border-t border-[var(--color-outline-variant)] pt-3">{children}</div> : null}
    </article>
  );
}

function ReasoningUserCard({ card }: { card: ReasoningWorkCard }) {
  const hasContent = card.content.trim().length > 0;
  const detail = hasContent ? card.content : "";
  const initializingHint = !card.ended && !hasContent && card.runCode === "pending"
    ? "System agent 正在初始化..."
    : "";
  const [expanded, setExpanded] = useState(false);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const duration = card.endMillis ? formatAguiDuration(Math.max(0, card.endMillis - card.firstMillis)) : "";

  useEffect(() => {
    if (!expanded || !detail) return;
    const element = contentRef.current;
    if (!element) return;
    const frame = window.requestAnimationFrame(() => {
      element.scrollTop = element.scrollHeight;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [detail, expanded]);

  return (
    <article aria-label="Thinking" className="rounded-md bg-white px-3 py-2 text-sm shadow-[0_8px_22px_rgba(15,23,42,0.06)]">
      <button
        className="flex w-full items-start justify-between gap-3 text-left"
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((current) => !current)}
      >
        <div className="flex min-w-0 items-start gap-2">
          <span className="mt-0.5 inline-flex size-7 shrink-0 items-center justify-center rounded-md bg-[var(--color-surface-container-low)] text-[var(--color-on-surface-variant)]">
            <BrainCircuit className={card.ended ? undefined : "animate-pulse"} size={16} />
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 items-center gap-2">
              <span className={cn("truncate font-semibold", card.ended ? undefined : "animate-pulse")}>
                {card.ended && duration ? `Thought for ${duration}` : "Thinking"}
              </span>
            </div>
            {initializingHint ? (
              <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                {initializingHint}
              </p>
            ) : null}
          </div>
        </div>
        <span className="mt-1 shrink-0 text-[var(--color-on-surface-variant)]">
          {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        </span>
      </button>
      {expanded && detail ? (
        <div ref={contentRef} className="mt-3 h-40 min-h-0 overflow-y-scroll overscroll-contain rounded-md bg-[var(--color-surface-container-lowest)] p-3">
          <MarkdownContent
            ariaLabel="Streaming reasoning markdown"
            className="min-h-0 py-0 leading-6"
            content={detail}
          />
        </div>
      ) : null}
    </article>
  );
}

function JsonBlock({ value }: { value: unknown }) {
  const preview = formatJsonBlockValue(value);
  if (!preview) return null;

  return (
    <pre className="overflow-x-auto whitespace-pre-wrap rounded-md bg-[var(--color-surface-container-lowest)] p-3 font-mono text-xs leading-5 text-[var(--color-on-surface)] [overflow-wrap:anywhere]">
      {preview}
    </pre>
  );
}

function ToolUserCard({ card }: { card: ToolWorkCard }) {
  const complete = card.ended || Boolean(card.result);
  const result = card.result || (complete ? "No result payload." : "");

  return (
    <AguiUserCardShell
      icon={complete ? <CheckCircle2 size={16} /> : <Hammer className="animate-pulse" size={16} />}
      meta={<Badge tone={complete ? "neutral" : "success"}>{complete ? "done" : "running"}</Badge>}
      title={toolSummaryLabel(card.toolName, card.args)}
      summary={(
        <p className="line-clamp-1 font-mono text-xs">
          {card.toolName} · {card.toolCallId}
        </p>
      )}
    >
      <div className="space-y-3">
        <section className="space-y-1.5">
          <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Parameters</h3>
          <JsonBlock value={card.args ?? { tool: card.toolName }} />
        </section>
        {complete ? (
          <section className="space-y-1.5">
            <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Result</h3>
            {result ? <JsonBlock value={result} /> : (
              <div className="rounded-md bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                No result payload.
              </div>
            )}
          </section>
        ) : (
          <section className="space-y-1.5">
            <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Log</h3>
            <div className="space-y-1 rounded-md bg-[var(--color-surface-container-lowest)] p-3 font-mono text-xs leading-5 text-[var(--color-on-surface)]">
              {card.logs.length ? card.logs.map((line, index) => (
                <p className="whitespace-pre-wrap" key={`${card.id}-log-${index}`}>{line}</p>
              )) : <p className="text-[var(--color-on-surface-variant)]">Waiting for tool activity...</p>}
            </div>
          </section>
        )}
      </div>
    </AguiUserCardShell>
  );
}

function AguiUserWorkRail({ cards }: { cards: AguiUserWorkCard[] }) {
  if (!cards.length) return null;

  return (
    <section className="space-y-2" aria-label="AG-UI user work cards">
      {cards.map((card) => (
        card.kind === "reasoning"
          ? <ReasoningUserCard card={card.value} key={`reasoning-${card.value.id}`} />
          : <ToolUserCard card={card.value} key={`tool-${card.value.id}`} />
      ))}
    </section>
  );
}

function AguiEventCard({ event }: { event: TranscriptAguiEvent }) {
  const type = aguiEventType(event);
  const summary = aguiEventSummary(event);

  return (
    <article className="rounded-md bg-white px-3 py-2 text-sm shadow-[0_8px_22px_rgba(15,23,42,0.06)]">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-md bg-[var(--color-surface-container-low)] text-[var(--color-on-surface-variant)]">
            {aguiEventIcon(type)}
          </span>
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <span className="truncate font-semibold">{aguiEventTitle(type)}</span>
              <Badge tone="neutral">{type}</Badge>
            </div>
            <p className="mt-1 line-clamp-2 text-[var(--color-on-surface-variant)]">{summary}</p>
          </div>
        </div>
        <span className="shrink-0 font-mono text-[11px] text-[var(--color-on-surface-variant)]">
          {formatTime(aguiEventTime(event))}
        </span>
      </div>
      <div className="mt-2 flex flex-wrap gap-2 font-mono text-[11px] text-[var(--color-on-surface-variant)]">
        <span>run {aguiEventRunCode(event)}</span>
        {"toolCallId" in event && typeof event.toolCallId === "string" ? <span>tool {event.toolCallId}</span> : null}
      </div>
    </article>
  );
}

function AguiEventRail({ events }: { events: TranscriptAguiEvent[] }) {
  if (!events.length) return null;

  return (
    <section className="space-y-2" aria-label="AG-UI debug event cards">
      {events.map((event, index) => (
        <AguiEventCard
          key={`${aguiEventType(event)}-${aguiEventRunCode(event)}-${aguiEventTime(event)}-${index}`}
          event={event}
        />
      ))}
    </section>
  );
}

function AguiRunWorkRail({
  events,
  pending,
  pendingStartedAt,
  preferences,
}: {
  events: TranscriptAguiEvent[];
  pending?: boolean;
  pendingStartedAt?: number | null;
  preferences: AguiDebugPreferences;
}) {
  const userCards = buildAguiUserWorkCards(events);
  const displayUserCards: AguiUserWorkCard[] = userCards.length || !pending
    ? userCards
    : [{
      kind: "reasoning",
      value: {
        content: "",
        ended: false,
        endMillis: null,
        firstMillis: pendingStartedAt ?? Date.now(),
        id: "pending-reasoning",
        runCode: "pending",
      },
    }];
  const debugEvents = events.filter((event) => shouldShowAguiEventType(aguiEventType(event), preferences));
  const terminalEvent = terminalAguiEvent(events);
  const terminalRunCode = terminalEvent ? aguiEventRunCode(terminalEvent) : "";
  const [expanded, setExpanded] = useState(true);

  useEffect(() => {
    setExpanded(!terminalRunCode);
  }, [terminalRunCode]);

  if (!terminalEvent && !displayUserCards.length && !debugEvents.length) return null;

  return (
    <section className="max-w-[920px] space-y-2" aria-label="AG-UI run work">
      {terminalEvent ? (
        <button
          aria-expanded={expanded}
          className="flex w-full items-center gap-3 py-1 text-xs font-medium text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)]"
          type="button"
          onClick={() => setExpanded((current) => !current)}
        >
          <span className="h-px flex-1 bg-[var(--color-outline-variant)]" />
          <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
            {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            Worked for {aguiWorkDuration(events)}
          </span>
          <span className="h-px flex-1 bg-[var(--color-outline-variant)]" />
        </button>
      ) : null}
      {!terminalEvent || expanded ? (
        <>
          <AguiUserWorkRail cards={displayUserCards} />
          <AguiEventRail events={debugEvents} />
        </>
      ) : null}
    </section>
  );
}

export function ChatManagementPage() {
  const toast = useToast();
  const { session: authSession } = useAuthSession();
  const { preferences: aguiDebugPreferences } = useAguiDebugPreferences();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const groups = useChatSessionGroupList(activeWorkspaceCode);
  const [sessionGroups, setSessionGroups] = useState<ChatSessionGroup[]>([]);
  const [activeSessionCode, setActiveSessionCode] = useState("");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [composer, setComposer] = useState("");
  const activeSession = findSession(sessionGroups, activeSessionCode) ?? firstSession(sessionGroups);
  const messages = useChatMessageListBySession(activeSession?.sessionCode ?? "");
  const aguiEvents = useChatAguiEvents(activeSession?.sessionCode ?? "");
  const [visibleMessages, setVisibleMessages] = useState<ChatMessage[]>([]);
  const [liveAguiEvents, setLiveAguiEvents] = useState<ChatAguiStreamEvent[]>([]);
  const [liveAguiPendingStartedAt, setLiveAguiPendingStartedAt] = useState<number | null>(null);
  const [liveAguiAnchorMessageCode, setLiveAguiAnchorMessageCode] = useState("");
  const [sending, setSending] = useState(false);
  const [creatingSession, setCreatingSession] = useState(false);
  const [chatModels, setChatModels] = useState<ModelRecord[]>([]);
  const [selectedModelCode, setSelectedModelCode] = useState("");
  const [sessionModelSelections, setSessionModelSelections] = useState<Record<string, string>>(() => readChatSessionModelSelections());
  const [modelsLoading, setModelsLoading] = useState(false);
  const [chatAgents, setChatAgents] = useState<ChatAgentOption[]>(() => [mainChatAgentOption]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(mainChatAgentOption.agentCode);
  const [agentsLoading, setAgentsLoading] = useState(false);
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const [renamingSession, setRenamingSession] = useState(false);
  const [runningSessionCode, setRunningSessionCode] = useState("");
  const [sessionActionPendingCode, setSessionActionPendingCode] = useState("");
  const [deleteTargetSession, setDeleteTargetSession] = useState<ChatSession | null>(null);
  const [unseenSessionCodes, setUnseenSessionCodes] = useState<Set<string>>(() => new Set());
  const [showScrollToBottom, setShowScrollToBottom] = useState(false);
  const activeSessionCodeRef = useRef("");
  const modelSelectionSessionRef = useRef("");
  const streamAbortControllerRef = useRef<AbortController | null>(null);
  const streamAbortRequestedRef = useRef(false);
  const autoStickToBottomRef = useRef(true);
  const messageScrollRef = useRef<HTMLElement | null>(null);
  const scrollFrameRef = useRef<number | null>(null);
  const visibleSessionCodeRef = useRef("");
  const effectiveAgentCode = selectedAgentCode || mainChatAgentOption.agentCode;
  const selectedAgent = chatAgents.find((agent) => agent.agentCode === effectiveAgentCode) ?? null;
  const agentScopedModels = useMemo(
    () => modelsForAgent(chatModels, selectedAgent),
    [chatModels, selectedAgent],
  );
  const selectedModel = agentScopedModels.find((model) => model.optionCode === selectedModelCode) ?? null;
  const selectedMainAgentModel = selectedModel?.modelCode
    ? chatModels.find((model) => model.modelCode === selectedModel.modelCode) ?? null
    : null;
  const hasActiveSession = Boolean(activeSession);
  const sessionStarted = Boolean(activeSession?.lastRunCode || visibleMessages.length > 0);
  const agentSelectionLocked = Boolean(activeSession && sessionStarted);
  const displayedAgentCode = sessionStarted
    ? (activeSession?.agentCode ?? selectedAgentCode ?? mainChatAgentOption.agentCode)
    : (selectedAgentCode || activeSession?.agentCode || mainChatAgentOption.agentCode);
  const modelEmptyLabel = isMainChatAgent(effectiveAgentCode)
    ? "No text model"
    : `No models for ${selectedAgent?.name ?? effectiveAgentCode}`;
  const currentUserDisplayName = authSession?.user.displayName ?? "";

  function scrollToLatestMessage(behavior: ScrollBehavior = "auto") {
    if (scrollFrameRef.current !== null) window.cancelAnimationFrame(scrollFrameRef.current);
    scrollFrameRef.current = window.requestAnimationFrame(() => {
      const element = messageScrollRef.current;
      if (!element) return;
      element.scrollTo({ behavior, top: element.scrollHeight });
      autoStickToBottomRef.current = true;
      setShowScrollToBottom(false);
      scrollFrameRef.current = null;
    });
  }

  function handleTranscriptScroll() {
    const element = messageScrollRef.current;
    if (!element) return;
    const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    autoStickToBottomRef.current = distanceFromBottom <= chatScrollBottomThreshold;
    setShowScrollToBottom(distanceFromBottom > chatScrollJumpButtonThreshold);
  }

  useEffect(() => {
    setSessionGroups(orderSessionGroups(groups));
  }, [groups]);

  useEffect(() => {
    let active = true;

    async function loadModels() {
      if (!activeWorkspaceCode) {
        setChatModels([]);
        setSelectedModelCode("");
        return;
      }

      setModelsLoading(true);
      try {
        const catalog = getServiceDataSource("model") === "api"
          ? await fetchWorkspaceModelCatalog(activeWorkspaceCode)
          : getWorkspaceModelCatalog(activeWorkspaceCode);
        if (!active) return;
        const models = availableChatModels(catalog.records);
        setChatModels(models);
      } catch (error) {
        if (active) {
          setChatModels([]);
          setSelectedModelCode("");
          toast.error(error instanceof Error ? error.message : "Workspace models could not be loaded", { title: "Model load failed" });
        }
      } finally {
        if (active) setModelsLoading(false);
      }
    }

    void loadModels();

    return () => {
      active = false;
    };
  }, [activeWorkspaceCode, toast]);

  useEffect(() => {
    let active = true;

    async function loadAgents() {
      if (!activeWorkspaceCode) {
        setChatAgents([mainChatAgentOption]);
        setSelectedAgentCode(mainChatAgentOption.agentCode);
        return;
      }

      setAgentsLoading(true);
      try {
        const records = getServiceDataSource("agent") === "api"
          ? await fetchWorkspaceSystemAgents(activeWorkspaceCode)
          : listWorkspaceSystemAgentsMock(activeWorkspaceCode);
        if (!active) return;
        const agents = availableChatAgents(records);
        setChatAgents(agents);
        setSelectedAgentCode((current) => (
          current && agents.some((agent) => agent.agentCode === current)
            ? current
            : mainChatAgentOption.agentCode
        ));
      } catch (error) {
        if (active) {
          setChatAgents([mainChatAgentOption]);
          setSelectedAgentCode(mainChatAgentOption.agentCode);
          toast.error(error instanceof Error ? error.message : "Workspace agents could not be loaded", { title: "Agent load failed" });
        }
      } finally {
        if (active) setAgentsLoading(false);
      }
    }

    void loadAgents();

    return () => {
      active = false;
    };
  }, [activeWorkspaceCode, toast]);

  useEffect(() => {
    const sessionAgentCode = activeSession?.agentCode ?? mainChatAgentOption.agentCode;
    setSelectedAgentCode((current) => {
      if (!chatAgents.some((agent) => agent.agentCode === sessionAgentCode)) {
        return current && chatAgents.some((agent) => agent.agentCode === current)
          ? current
          : mainChatAgentOption.agentCode;
      }
      return sessionAgentCode;
    });
  }, [activeSession?.agentCode, activeSession?.sessionCode, chatAgents]);

  useEffect(() => {
    const sessionCode = activeSession?.sessionCode ?? "";
    const switchedSession = modelSelectionSessionRef.current !== sessionCode;

    if (switchedSession) {
      modelSelectionSessionRef.current = sessionCode;
      const persistedOptionCode = sessionCode ? sessionModelSelections[sessionCode] : "";
      const defaultModel = defaultChatModelOption(agentScopedModels, chatModels);
      const restoredOptionCode = persistedOptionCode && agentScopedModels.some((model) => model.optionCode === persistedOptionCode)
        ? persistedOptionCode
        : (defaultModel?.optionCode ?? "");
      setSelectedModelCode(restoredOptionCode);
      return;
    }

    setSelectedModelCode((current) => {
      if (current && agentScopedModels.some((model) => model.optionCode === current)) {
        return current;
      }
      const persistedOptionCode = sessionCode ? sessionModelSelections[sessionCode] : "";
      if (persistedOptionCode && agentScopedModels.some((model) => model.optionCode === persistedOptionCode)) {
        return persistedOptionCode;
      }
      return defaultChatModelOption(agentScopedModels, chatModels)?.optionCode ?? "";
    });
  }, [activeSession?.sessionCode, agentScopedModels, chatModels, sessionModelSelections]);

  useEffect(() => {
    const sessionCode = activeSession?.sessionCode;
    if (!sessionCode || !selectedModelCode) return;
    const activeSessionAgentCode = activeSession.agentCode ?? mainChatAgentOption.agentCode;
    if (selectedAgentCode !== activeSessionAgentCode) return;
    if (!agentScopedModels.some((model) => model.optionCode === selectedModelCode)) return;
    setSessionModelSelections((current) => (
      current[sessionCode] === selectedModelCode
        ? current
        : {
          ...current,
          [sessionCode]: selectedModelCode,
        }
    ));
  }, [
    activeSession?.agentCode,
    activeSession?.sessionCode,
    agentScopedModels,
    selectedAgentCode,
    selectedModelCode,
  ]);

  useEffect(() => {
    writeChatSessionModelSelections(sessionModelSelections);
  }, [sessionModelSelections]);

  useEffect(() => {
    if (!activeSessionCode && activeSession) {
      setActiveSessionCode(activeSession.sessionCode);
    }
  }, [activeSession, activeSessionCode]);

  useEffect(() => {
    activeSessionCodeRef.current = activeSessionCode;
  }, [activeSessionCode]);

  useEffect(() => {
    const htmlElement = document.documentElement;
    const bodyElement = document.body;

    htmlElement.classList.add("tron-chat-scroll-lock");
    bodyElement.classList.add("tron-chat-scroll-lock");

    return () => {
      htmlElement.classList.remove("tron-chat-scroll-lock");
      bodyElement.classList.remove("tron-chat-scroll-lock");
    };
  }, []);

  useEffect(() => () => {
    if (scrollFrameRef.current !== null) window.cancelAnimationFrame(scrollFrameRef.current);
    streamAbortControllerRef.current?.abort();
  }, []);

  useEffect(() => {
    autoStickToBottomRef.current = true;
    setShowScrollToBottom(false);
    scrollToLatestMessage("auto");
  }, [activeSession?.sessionCode]);

  useEffect(() => {
    const sessionCode = activeSession?.sessionCode ?? "";
    setVisibleMessages(sessionCode ? messages.filter((message) => message.sessionCode === sessionCode) : []);
    if (visibleSessionCodeRef.current !== sessionCode) {
      visibleSessionCodeRef.current = sessionCode;
      setLiveAguiEvents([]);
      setLiveAguiPendingStartedAt(null);
      setLiveAguiAnchorMessageCode("");
    }
  }, [activeSession?.sessionCode, messages]);

  const transcriptAguiEvents = useMemo<TranscriptAguiEvent[]>(() => eventsForLatestAguiRun(liveAguiEvents), [liveAguiEvents]);
  const liveAguiPending = Boolean(
    sending
    && activeSession
    && runningSessionCode === activeSession.sessionCode
    && !liveAguiEvents.length,
  );
  const canAbortCurrentSession = Boolean(
    sending
    && activeSession
    && runningSessionCode === activeSession.sessionCode,
  );
  const liveAguiAnchorIndex = useMemo(() => {
    if (!transcriptAguiEvents.length && !liveAguiPending) return -1;

    const anchoredIndex = visibleMessages.findIndex((message) => message.messageCode === liveAguiAnchorMessageCode);
    if (anchoredIndex >= 0) return anchoredIndex;

    for (let index = visibleMessages.length - 1; index >= 0; index -= 1) {
      if (visibleMessages[index].role === "USER") return index;
    }

    return -1;
  }, [liveAguiAnchorMessageCode, liveAguiPending, transcriptAguiEvents.length, visibleMessages]);

  const rightPanel = useMemo(() => {
    if (!hasActiveSession) return undefined;
    return (
      <RightContextSidebar
        events={aguiEvents}
        messages={visibleMessages}
        model={selectedMainAgentModel}
        selectedModel={selectedModel}
        session={activeSession}
      />
    );
  }, [activeSession, aguiEvents, hasActiveSession, selectedMainAgentModel, selectedModel, visibleMessages]);

  useConsoleShellConfig({ rightPanel });

  useEffect(() => {
    if (autoStickToBottomRef.current) scrollToLatestMessage("auto");
    else handleTranscriptScroll();
  }, [transcriptAguiEvents, visibleMessages]);

  function handleSessionSelect(sessionCode: string) {
    setActiveSessionCode(sessionCode);
    setLiveAguiPendingStartedAt(null);
    setUnseenSessionCodes((current) => {
      if (!current.has(sessionCode)) return current;
      const next = new Set(current);
      next.delete(sessionCode);
      return next;
    });
  }

  function handleModelSelect(optionCode: string) {
    setSelectedModelCode(optionCode);
    const sessionCode = activeSession?.sessionCode;
    if (!sessionCode) return;
    setSessionModelSelections((current) => (
      current[sessionCode] === optionCode
        ? current
        : {
          ...current,
          [sessionCode]: optionCode,
        }
    ));
  }

  function handleAbortSend() {
    if (!sending) return;
    streamAbortRequestedRef.current = true;
    streamAbortControllerRef.current?.abort();
  }

  function forgetSessionModelSelection(sessionCode: string) {
    setSessionModelSelections((current) => {
      if (!(sessionCode in current)) return current;
      const next = { ...current };
      delete next[sessionCode];
      return next;
    });
  }

  async function handleSend() {
    if (sending) return;
    if (!activeSession) return;
    const content = composer.trim();
    if (!content) {
      toast.warning("Message content is required", { title: "Nothing to send" });
      return;
    }
    if (!selectedModel) {
      toast.warning("Select a model before sending", { title: "Model required" });
      return;
    }
    const sendInput: SendChatMessageInput = {
      agentCode: effectiveAgentCode,
      appCode: activeSession.appCode,
      content,
      taskCode: activeSession.taskCode,
      workspaceCode: activeSession.workspaceCode,
    };
    if (selectedModel.source === "system-agent") {
      sendInput.systemModelId = selectedModel.modelName;
    } else {
      sendInput.modelCode = selectedModel.modelCode;
    }
    const sendingSessionCode = activeSession.sessionCode;
    const abortController = new AbortController();
    streamAbortRequestedRef.current = false;
    streamAbortControllerRef.current = abortController;
    const optimisticUserCode = `LOCAL-${Date.now()}-USER`;
    const optimisticAssistantCode = `LOCAL-${Date.now()}-ASSISTANT`;
    const optimisticCreatedAt = new Date().toISOString();
    const optimisticUserMessage: ChatMessage = {
      actor: "User",
      channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content,
      createdAt: optimisticCreatedAt,
      id: -Date.now(),
      messageCode: optimisticUserCode,
      role: "USER",
      runCode: null,
      sessionCode: sendingSessionCode,
    };
    const optimisticAssistantMessage: ChatMessage = {
      actor: "Assistant",
      channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content: "",
      createdAt: optimisticCreatedAt,
      id: -Date.now() - 1,
      messageCode: optimisticAssistantCode,
      role: "ASSISTANT",
      runCode: null,
      sessionCode: sendingSessionCode,
    };
    setSending(true);
    setRunningSessionCode(sendingSessionCode);
    setLiveAguiPendingStartedAt(Date.now());
    setComposer("");
    setLiveAguiEvents([]);
    setLiveAguiAnchorMessageCode(optimisticUserCode);
    if (activeSessionCodeRef.current === sendingSessionCode) {
      setVisibleMessages((current) => [
        ...current,
        optimisticUserMessage,
      ]);
      autoStickToBottomRef.current = true;
      setShowScrollToBottom(false);
      scrollToLatestMessage("auto");
    }
    try {
      const result = await streamChatMessage(
        activeSession,
        sendInput,
        {
          onAguiEvent: (event) => {
            if (event.threadId && event.threadId !== sendingSessionCode) return;
            setLiveAguiPendingStartedAt(null);
            setLiveAguiEvents((current) => [...current, event]);
          },
          onDelta: (delta) => {
            if (activeSessionCodeRef.current !== sendingSessionCode) return;
            setVisibleMessages((current) => {
              const assistantExists = current.some((message) => message.messageCode === optimisticAssistantCode);
              if (!assistantExists) {
                return [
                  ...current,
                  { ...optimisticAssistantMessage, content: delta },
                ];
              }
              return current.map((message) => (
                message.messageCode === optimisticAssistantCode
                  ? { ...message, content: `${message.content}${delta}` }
                  : message
              ));
            });
          },
        },
        abortController.signal,
      );
      setSessionGroups((current) => replaceSession(current, result.session));
      if (activeSessionCodeRef.current === result.session.sessionCode) {
        const resultUserMessage = result.messages.find((message) => message.role === "USER") ?? null;
        setLiveAguiAnchorMessageCode(resultUserMessage?.messageCode ?? optimisticUserCode);
        setVisibleMessages((current) => [
          ...current.filter((message) => (
            message.messageCode !== optimisticUserCode
            && message.messageCode !== optimisticAssistantCode
          )),
          ...result.messages,
        ]);
      }
      setUnseenSessionCodes((current) => {
        const next = new Set(current);
        if (activeSessionCodeRef.current === result.session.sessionCode) next.delete(result.session.sessionCode);
        else next.add(result.session.sessionCode);
        return next;
      });
    } catch (error) {
      const stopped = streamAbortRequestedRef.current || isAbortError(error);
      if (stopped) {
        if (activeSessionCodeRef.current === sendingSessionCode) {
          setVisibleMessages((current) => current.filter((item) => (
            item.messageCode !== optimisticAssistantCode || Boolean(item.content.trim())
          )));
        }
        toast.info("Current response has been stopped", { title: "Chat stopped" });
        return;
      }
      const rawMessage = error instanceof Error ? error.message : "Chat send failed";
      const toastMessage = chatSendErrorToastMessage(rawMessage);
      if (activeSessionCodeRef.current === sendingSessionCode) {
        setVisibleMessages((current) => current.filter((item) => (
          item.messageCode !== optimisticAssistantCode || Boolean(item.content.trim())
        )));
      }
      toast.error(toastMessage, { title: "Chat send failed" });
    } finally {
      setSending(false);
      setRunningSessionCode("");
      setLiveAguiPendingStartedAt(null);
      if (streamAbortControllerRef.current === abortController) {
        streamAbortControllerRef.current = null;
      }
      streamAbortRequestedRef.current = false;
    }
  }

  async function handleCreateSession() {
    if (!activeWorkspaceCode) {
      toast.warning("Create a workspace before starting a chat session", { title: "No workspace selected" });
      return;
    }

    setCreatingSession(true);
    try {
      const session = await createChatSession({
        agentCode: selectedAgentCode || mainChatAgentOption.agentCode,
        title: "New chat",
        workspaceCode: activeWorkspaceCode,
      });
      setSessionGroups((current) => insertSession(current, session));
      handleSessionSelect(session.sessionCode);
      setVisibleMessages([]);
      setComposer("");
      toast.success("Session created", { title: session.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Create session failed";
      toast.error(message, { title: "Create session failed" });
    } finally {
      setCreatingSession(false);
    }
  }

  async function handleArchiveSession(session: ChatSession) {
    setSessionActionPendingCode(session.sessionCode);
    try {
      const archived = await archiveChatSession(session.sessionCode);
      const nextGroups = removeSession(sessionGroups, archived.sessionCode);
      setSessionGroups(nextGroups);
      setUnseenSessionCodes((current) => {
        if (!current.has(archived.sessionCode)) return current;
        const next = new Set(current);
        next.delete(archived.sessionCode);
        return next;
      });
      if (activeSessionCode === archived.sessionCode) {
        handleSessionSelect(firstSession(nextGroups)?.sessionCode ?? "");
        setVisibleMessages([]);
      }
      forgetSessionModelSelection(archived.sessionCode);
      toast.success("Session archived", { title: archived.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Archive session failed";
      toast.error(message, { title: "Archive session failed" });
    } finally {
      setSessionActionPendingCode("");
    }
  }

  async function confirmDeleteSession() {
    if (!deleteTargetSession) return;
    const session = deleteTargetSession;

    setSessionActionPendingCode(session.sessionCode);
    try {
      const deleted = await deleteChatSession(session.sessionCode);
      const nextGroups = removeSession(sessionGroups, deleted.sessionCode);
      setSessionGroups(nextGroups);
      setUnseenSessionCodes((current) => {
        if (!current.has(deleted.sessionCode)) return current;
        const next = new Set(current);
        next.delete(deleted.sessionCode);
        return next;
      });
      if (activeSessionCode === deleted.sessionCode) {
        handleSessionSelect(firstSession(nextGroups)?.sessionCode ?? "");
        setVisibleMessages([]);
      }
      forgetSessionModelSelection(deleted.sessionCode);
      toast.success("Session deleted", { title: deleted.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Delete session failed";
      toast.error(message, { title: "Delete session failed" });
    } finally {
      setSessionActionPendingCode("");
      setDeleteTargetSession((current) => (
        current?.sessionCode === session.sessionCode ? null : current
      ));
    }
  }

  async function handleTitleSubmit() {
    if (!activeSession || renamingSession) return;
    const title = titleDraft.trim();
    if (!title) {
      toast.warning("Session title is required", { title: "Rename skipped" });
      return;
    }
    if (title === activeSession.title) {
      setEditingTitle(false);
      return;
    }

    setRenamingSession(true);
    try {
      const session = await updateChatSession(activeSession.sessionCode, { title });
      setSessionGroups((current) => replaceSession(current, session));
      setEditingTitle(false);
      toast.success("Session renamed", { title: session.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Rename session failed";
      toast.error(message, { title: "Rename session failed" });
    } finally {
      setRenamingSession(false);
    }
  }

  async function handleSidebarRenameSession(session: ChatSession, title: string) {
    if (!title.trim()) {
      toast.warning("Session title is required", { title: "Rename skipped" });
      return;
    }

    setSessionActionPendingCode(session.sessionCode);
    try {
      const renamed = await updateChatSession(session.sessionCode, { title });
      setSessionGroups((current) => replaceSession(current, renamed));
      toast.success("Session renamed", { title: renamed.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Rename session failed";
      toast.error(message, { title: "Rename session failed" });
    } finally {
      setSessionActionPendingCode("");
    }
  }

  return (
    <div className="relative z-0 flex h-full min-h-0 min-w-0 overflow-hidden overscroll-none bg-white">
      <SessionSidebar
        activeSessionCode={activeSession?.sessionCode ?? ""}
        collapsed={sidebarCollapsed}
        creatingSession={creatingSession}
        groups={sessionGroups}
        runningSessionCode={runningSessionCode}
        sessionActionPendingCode={sessionActionPendingCode}
        unseenSessionCodes={unseenSessionCodes}
        onArchiveSession={(session) => void handleArchiveSession(session)}
        onCollapseToggle={() => setSidebarCollapsed((value) => !value)}
        onCreateSession={() => void handleCreateSession()}
        onDeleteSession={setDeleteTargetSession}
        onRenameSession={handleSidebarRenameSession}
        onSessionSelect={handleSessionSelect}
      />
      <main className="flex min-h-0 min-w-0 flex-1 flex-col">
        <header className="flex h-14 items-center justify-between px-5 shadow-[0_8px_24px_rgba(15,23,42,0.035)]">
          <div className="min-w-0">
            {editingTitle && activeSession ? (
              <input
                autoFocus
                className="h-7 w-[min(420px,70vw)] rounded-md border border-[var(--color-focus-ring)] bg-white px-2 text-base font-semibold text-[var(--color-on-surface)] outline-none"
                disabled={renamingSession}
                value={titleDraft}
                onBlur={() => {
                  if (!renamingSession) setEditingTitle(false);
                }}
                onChange={(event) => setTitleDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    void handleTitleSubmit();
                  }
                  if (event.key === "Escape") {
                    setEditingTitle(false);
                  }
                }}
              />
            ) : (
              <h1
                className={cn(
                  "truncate text-base font-semibold",
                  activeSession && "cursor-text rounded-sm focus:outline-none focus:ring-2 focus:ring-[var(--color-focus-ring)]",
                )}
                tabIndex={activeSession ? 0 : undefined}
                title={activeSession ? "Double-click to rename" : undefined}
                onDoubleClick={() => {
                  if (!activeSession) return;
                  setTitleDraft(activeSession.title);
                  setEditingTitle(true);
                }}
                onKeyDown={(event) => {
                  if (!activeSession || event.key !== "Enter") return;
                  setTitleDraft(activeSession.title);
                  setEditingTitle(true);
                }}
              >
                {activeSession?.title ?? "Chat Management"}
              </h1>
            )}
            {hasActiveSession ? (
              <p className="mt-0.5 flex items-center gap-2 text-xs text-[var(--color-on-surface-variant)]">
                <CircleDot size={12} />
                {activeSession?.sessionCode}
                <span
                  className={cn(
                    "inline-flex items-center rounded-full border px-1.5 py-0.5 text-[10px] font-semibold uppercase leading-none",
                    sessionAgentTagTone(displayedAgentCode),
                  )}
                >
                  {sessionAgentTagLabel(displayedAgentCode)}
                </span>
              </p>
            ) : (
              <p className="mt-0.5 text-xs text-[var(--color-on-surface-variant)]">
                Select a session from the left to continue.
              </p>
            )}
          </div>
        </header>

        {hasActiveSession ? (
          <>
            <section
              aria-label="Chat transcript"
              className="flex-1 overflow-y-auto bg-[var(--color-surface-container-low)] px-5 py-5"
              ref={messageScrollRef}
              onScroll={handleTranscriptScroll}
            >
              <div className="mx-auto flex max-w-6xl flex-col gap-4">
                {visibleMessages.length ? visibleMessages.map((message) => (
                  <Fragment key={message.messageCode}>
                    <ChatBubble
                      currentUserDisplayName={currentUserDisplayName}
                      message={message}
                    />
                    {liveAguiAnchorIndex >= 0 && visibleMessages[liveAguiAnchorIndex]?.messageCode === message.messageCode ? (
                      <AguiRunWorkRail
                        events={transcriptAguiEvents}
                        pending={liveAguiPending}
                        pendingStartedAt={liveAguiPendingStartedAt}
                        preferences={aguiDebugPreferences}
                      />
                    ) : null}
                  </Fragment>
                )) : activeWorkspaceCode ? (
                  <div className="flex min-h-[360px] items-center justify-center rounded-md bg-[var(--color-surface-container-lowest)] text-sm text-[var(--color-on-surface-variant)] shadow-[inset_0_0_0_1px_rgba(15,23,42,0.05)]">
                    No persisted messages in this session.
                  </div>
                ) : (
                  <div className="flex min-h-[360px] items-center justify-center rounded-md bg-[var(--color-surface-container-lowest)] text-sm text-[var(--color-on-surface-variant)] shadow-[inset_0_0_0_1px_rgba(15,23,42,0.05)]">
                    Select or create a workspace before loading chat sessions.
                  </div>
                )}
              </div>
            </section>

            <footer className="relative bg-white px-4 py-3 shadow-[0_-10px_28px_rgba(15,23,42,0.04)]">
              {showScrollToBottom ? (
                <button
                  aria-label="Jump to latest message"
                  className="absolute left-1/2 top-0 z-20 inline-flex size-8 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] shadow-[0_10px_26px_rgba(15,23,42,0.18)] ring-1 ring-[var(--color-outline-variant)] transition hover:bg-[var(--color-hover-surface)] focus:outline-none focus:ring-2 focus:ring-[var(--color-focus-ring)]"
                  type="button"
                  onClick={() => scrollToLatestMessage("smooth")}
                >
                  <ArrowDown size={16} />
                </button>
              ) : null}
              <div className="flex min-h-28 w-full flex-col rounded-md bg-[var(--color-surface-container-lowest)] px-3 py-3 shadow-[inset_0_0_0_1px_rgba(15,23,42,0.055)]">
                <TextArea
                  className="min-h-16 w-full flex-1 resize-none rounded-none border-0 bg-transparent p-0 text-sm shadow-none focus:border-transparent focus:ring-0"
                  placeholder="Enter to send, Shift+Enter for newline"
                  value={composer}
                  onChange={(event) => setComposer(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key !== "Enter" || event.nativeEvent.isComposing) return;
                    event.preventDefault();
                    if (event.shiftKey) {
                      setComposer(insertNewlineAtCursor(event.currentTarget));
                      return;
                    }
                    void handleSend();
                  }}
                />
                <div className="mt-2 flex flex-wrap items-center gap-2">
                  <div className="flex min-w-0 max-w-full items-center gap-1">
                    {agentSelectionLocked ? null : (
                      <div className="min-w-0 max-w-[9.5rem] shrink">
                        <ChatAgentSelector
                          locked={agentSelectionLocked}
                          loading={agentsLoading}
                          agents={chatAgents}
                          selectedAgentCode={selectedAgentCode}
                          onSelect={setSelectedAgentCode}
                        />
                      </div>
                    )}
                    <div
                      className={cn(
                        "min-w-0 max-w-full",
                        agentSelectionLocked ? "w-[min(100%,16rem)]" : "w-[min(100%,14rem)]",
                      )}
                    >
                      <ChatModelSelector
                        emptyLabel={modelEmptyLabel}
                        loading={modelsLoading}
                        models={agentScopedModels}
                        selectedModelCode={selectedModelCode}
                        onSelect={handleModelSelect}
                      />
                    </div>
                  </div>
                  <Button
                    className={cn(
                      "ml-auto shrink-0",
                      canAbortCurrentSession && "bg-[var(--color-error)] text-[var(--color-on-error)] hover:bg-[var(--color-error-container)]",
                    )}
                    disabled={!canAbortCurrentSession && (sending || !activeSession || !selectedModel)}
                    size="sm"
                    onClick={() => {
                      if (canAbortCurrentSession) {
                        handleAbortSend();
                        return;
                      }
                      void handleSend();
                    }}
                  >
                    {canAbortCurrentSession ? <Square size={14} /> : <Send size={15} />}
                    {canAbortCurrentSession ? "Stop" : "Send"}
                  </Button>
                </div>
              </div>
            </footer>
          </>
        ) : (
          <section
            aria-label="Chat onboarding"
            className="flex flex-1 items-center justify-center bg-[var(--color-surface-container-low)] px-5 py-8"
          >
            <div className="w-full max-w-2xl rounded-lg bg-[var(--color-surface-container-lowest)] p-8 text-center shadow-[inset_0_0_0_1px_rgba(15,23,42,0.05)]">
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">
                No session selected
              </h2>
              <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
                Create a new session or pick one from the left panel to view chat details and start messaging.
              </p>
            </div>
          </section>
        )}
      </main>
      <ConfirmDialog
        ariaLabel="Delete chat session confirmation"
        cancelText="Cancel"
        confirmText={sessionActionPendingCode === deleteTargetSession?.sessionCode ? "Deleting..." : "Delete"}
        description={deleteTargetSession
          ? `Delete "${deleteTargetSession.title}" permanently? This action cannot be undone.`
          : undefined}
        open={Boolean(deleteTargetSession)}
        pending={sessionActionPendingCode === deleteTargetSession?.sessionCode}
        title="Delete session"
        tone="danger"
        onCancel={() => {
          if (sessionActionPendingCode === deleteTargetSession?.sessionCode) return;
          setDeleteTargetSession(null);
        }}
        onConfirm={() => void confirmDeleteSession()}
      />
    </div>
  );
}
