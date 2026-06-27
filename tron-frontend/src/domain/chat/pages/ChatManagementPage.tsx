import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate, useSearchParams } from "react-router";
import {
  AlarmClock,
  ArrowDown,
  Archive,
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
  ListTodo,
  LoaderCircle,
  MessageSquareText,
  Pencil,
  Plus,
  RefreshCw,
  Send,
  Square,
  Terminal,
  UserRound,
  Wrench,
  type LucideIcon,
  X,
  MessageCircleMore,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import {
  ConfirmDialog,
  MarkdownContent,
  useWorkspaceSpaces,
  useConsoleShellConfig,
  useDismissibleLayer,
  useToast,
  PageView,
  type TargetPageFrameInspectionResult,
} from "@/shared";
import { cn } from "@/shared/lib/utils";
import {
  archiveChatSession,
  createChatSession,
  deleteChatSession,
  streamChatMessage,
  updateChatSession,
  cancelChatSessionGoal,
  clearChatSessionGoal,
  useChatAguiEvents,
  useChatAguiProtocolEvents,
  pauseChatSessionGoal,
  resumeChatSessionGoal,
  createChatCompaction,
  setChatSessionGoal,
  useChatLatestCompactionState,
  useChatMessageListBySession,
  useChatSessionGoalState,
  useChatSessionTodosState,
  useChatSessionGroupList,
} from "@/domain/chat/hooks";
import { useAuthSession } from "@/domain/auth";
import { useTranslation } from "react-i18next";
import { fetchWorkspaceSystemAgents } from "@/domain/agent/api";
import { listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import type { SystemAgentModelRecord, WorkspaceSystemAgentRecord } from "@/domain/agent/model";
import { fetchWorkspaceModelCatalog } from "@/domain/model/api";
import { getWorkspaceModelCatalog } from "@/domain/model/mock";
import type { ModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  resolveAguiDebugEventTypes,
  shouldShowAguiEventType,
  useAguiDebugPreferences,
  type AguiDebugPreferences,
} from "@/domain/settings/model/aguiDebugPreferences";
import { useAguiWorkReplayPreferences } from "@/domain/settings/model/aguiWorkReplayPreferences";
import { TronChatPanel } from "@tron/chat-react";
import { deriveReasoningPreview, truncateLabel } from "./chatReasoningPreview";
import { inferChatAgentCodeFromModel } from "../model/chatAgentCode";
import type {
  ChatAguiEvent,
  ChatAguiStreamEvent,
  ChatMessage,
  ChatSessionCompaction,
  ChatSessionGoalView,
  ChatSession,
  ChatSessionGroup,
  ChatSessionTodo,
  ChatMessageAttachment,
  ChatTokenEstimateSnapshot,
  ChatTokenUsage,
  ChatUsageAuditSnapshot,
  SendChatMessageInput,
} from "@/domain/chat/model";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useInspection } from "../hooks/useInspection";
import DraggleLine from "@/shared/ui/DraggleLine";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function useStableCallback<TArgs extends unknown[], TResult>(
  callback: (...args: TArgs) => TResult,
): (...args: TArgs) => TResult {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;
  return useCallback((...args: TArgs) => callbackRef.current(...args), []);
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

function shouldAutoFocusChatComposer() {
  if (typeof window === "undefined") return false;
  if (typeof window.matchMedia !== "function") {
    return window.innerWidth >= 1024;
  }
  return window.matchMedia("(min-width: 1024px)").matches
    || window.matchMedia("(hover: hover) and (pointer: fine)").matches;
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

type ChatSessionSourceTab = "chat" | "automation" | "task";

const chatSessionSourceTabs: { id: ChatSessionSourceTab; icon: LucideIcon; label: string }[] = [
  { id: "chat", icon: MessageSquareText, label: "Chat sessions" },
  { id: "automation", icon: AlarmClock, label: "Automation sessions" },
  { id: "task", icon: ListTodo, label: "Task sessions" },
];
const defaultChatSessionSourceTab: ChatSessionSourceTab = "chat";
const chatSessionSourceTabStorageKey = "tron.chat.activeSourceTab";
const emptyLiveAguiEvents: ChatAguiStreamEvent[] = [];
const emptyLiveAguiAnchorMessageCodes: Record<string, string> = {};

function ChatInspectionAttachmentChip({
  attachment,
  onRemove,
}: {
  attachment: ChatMessageAttachment;
  onRemove: () => void;
}) {
  return (

    <div className="group relative">
      <Button variant="outline" size="sm" className="hover:bg-background!">
        <MessageCircleMore />
        <p className="text-sm">注释</p>
      </Button>
      <Button variant="outline" onClick={onRemove} size="sm" className="cursor-pointer rounded-full absolute right-[2px] top-[2px] w-6 h-6 hidden group-hover:flex"><X size={2} /></Button>
      <Card size="sm" className="pointer-events-none absolute hidden bottom-[calc(100%+0.5rem)] left-0 z-20 group-hover:block --card-spacing:--spacing(4)">
        <CardContent className="space-x-1 flex item-center">
          <div className="w-6 h-6 rounded-sm overflow-hidden flex-none">
            {attachment.imageUrl ? (
              <img
                alt="Inspection attachment preview"
                className="object-cover w-full h-full"
                src={attachment.imageUrl}
              />
            ) : null}
          </div>
          <p className="max-w-20 truncate whitespace-nowrap text-[var(--color-on-surface)]">{attachment.summary}</p>
        </CardContent>
      </Card>
    </div>
  );
}

function normalizedSessionSource(value: string | null | undefined) {
  return value?.trim().toLowerCase() ?? "";
}

function parseChatSessionSourceTab(value: string | null | undefined): ChatSessionSourceTab | undefined {
  const source = normalizedSessionSource(value);
  if (source === "chat" || source === "automation" || source === "task") return source;
  return undefined;
}

function writeStoredChatSessionSourceTab(sourceTab: ChatSessionSourceTab) {
  if (typeof window === "undefined") return;
  window.sessionStorage.setItem(chatSessionSourceTabStorageKey, sourceTab);
}

function readInspectionString(dom: Record<string, unknown>, key: string) {
  const value = dom[key];
  return typeof value === "string" ? value : "";
}

function inspectionSubmitMode(result: TargetPageFrameInspectionResult) {
  return readInspectionString(result.dom, "submitMode") === "compose" ? "compose" : "send";
}

function inspectionMessageContent(result: TargetPageFrameInspectionResult) {
  return result.remark.trim() || readInspectionString(result.dom, "remark").trim();
}

function sourceTabFromValues(
  sourceType: string | null | undefined,
  sourceCode: string | null | undefined,
  taskCode: string | null | undefined,
  groupTitle?: string,
): ChatSessionSourceTab {
  const source = normalizedSessionSource(sourceType);
  if (source === "task") return "task";
  if (source === "automation") return "automation";
  if (taskCode?.trim()) return "task";

  const code = normalizedSessionSource(sourceCode);
  const title = normalizedSessionSource(groupTitle);
  if (code.startsWith("automation") || title === "automation" || title.startsWith("automation:")) return "automation";
  return "chat";
}

function sourceTabForGroup(group: ChatSessionGroup): ChatSessionSourceTab {
  const firstSession = group.sessions[0];
  return sourceTabFromValues(
    group.sourceType ?? firstSession?.sourceType,
    group.sourceCode ?? firstSession?.sourceCode,
    group.taskCode ?? firstSession?.taskCode,
    group.taskTitle,
  );
}

function sourceTabForSessionCode(groups: ChatSessionGroup[], sessionCode: string): ChatSessionSourceTab | undefined {
  const group = groups.find((item) => item.sessions.some((session) => session.sessionCode === sessionCode));
  return group ? sourceTabForGroup(group) : undefined;
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

function firstSessionForSourceTab(groups: ChatSessionGroup[], sourceTab: ChatSessionSourceTab) {
  return groups
    .filter((group) => sourceTabForGroup(group) === sourceTab)
    .flatMap((group) => group.sessions)[0];
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

function sessionGroupKey(group: ChatSessionGroup) {
  return [
    group.sourceType ?? "Manual",
    group.sourceCode ?? "",
    group.taskCode ?? "",
    group.taskTitle,
  ].join(":");
}

function sessionGroupKeyForSession(session: ChatSession) {
  return [
    session.sourceType ?? "Manual",
    session.sourceCode ?? "",
    session.taskCode ?? "",
    session.taskCode ? session.taskCode : "Chat",
  ].join(":");
}

function insertSession(groups: ChatSessionGroup[], session: ChatSession): ChatSessionGroup[] {
  const groupKey = sessionGroupKeyForSession(session);
  let inserted = false;
  const nextGroups = groups.map((group) => {
    const currentKey = sessionGroupKey(group);
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
      sourceCode: session.sourceCode,
      sourceType: session.sourceType,
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

/**
 * Clear MainAgentChatWindow's persisted session from localStorage if its sessionCode
 * matches the one that was just deleted. This prevents the floating agent chat window
 * from restoring a stale (deleted) session on next open.
 */
function forgetMainAgentPersistedSession(workspaceCode: string, sessionCode: string) {
  const storageKey = `tron.main-agent.session:${workspaceCode.trim() || "__default__"}`;
  try {
    const raw = localStorage.getItem(storageKey);
    if (!raw) return;
    const persisted = JSON.parse(raw) as { sessionCode?: string };
    if (persisted?.sessionCode === sessionCode) {
      localStorage.removeItem(storageKey);
    }
  } catch {
    // Ignore parse errors from corrupt localStorage data
  }
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

function normalizeUsageEstimate(parsed: Partial<ChatTokenEstimateSnapshot> | null | undefined): ChatTokenEstimateSnapshot | null {
  if (typeof parsed?.estimatedInputTokens !== "number") return null;
  if (typeof parsed.messageTokenEstimate !== "number") return null;
  if (typeof parsed.toolSchemaTokenEstimate !== "number") return null;
  return {
    checkpointCode: parsed.checkpointCode ?? null,
    contextWindowLimit: tokenField(parsed.contextWindowLimit),
    effectiveInputBudget: tokenField(parsed.effectiveInputBudget),
    estimatedInputTokens: parsed.estimatedInputTokens,
    messageCount: tokenField(parsed.messageCount) ?? 0,
    messageTokenEstimate: parsed.messageTokenEstimate,
    modelCode: parsed.modelCode ?? "",
    modelName: parsed.modelName ?? "",
    modelProtocol: parsed.modelProtocol ?? "",
    outputTokenReserve: tokenField(parsed.outputTokenReserve),
    overBudget: Boolean(parsed.overBudget),
    compactionTriggerReached: Boolean(parsed.compactionTriggerReached),
    partCount: tokenField(parsed.partCount) ?? 0,
    rebased: Boolean(parsed.rebased),
    toolSchemaTokenEstimate: parsed.toolSchemaTokenEstimate,
    usageConfidence: parsed.usageConfidence ?? "estimated",
    usageSource: parsed.usageSource ?? "local_estimate",
  };
}

function normalizeTokenUsage(parsed: Partial<ChatTokenUsage> | null | undefined): ChatTokenUsage | null {
  if (typeof parsed?.inputTokens !== "number") return null;
  if (typeof parsed.outputTokens !== "number") return null;
  if (typeof parsed.totalTokens !== "number") return null;
  return {
    cachedInputTokens: tokenField(parsed.cachedInputTokens),
    inputTokens: parsed.inputTokens,
    outputTokens: parsed.outputTokens,
    reasoningTokens: tokenField(parsed.reasoningTokens),
    totalTokens: parsed.totalTokens,
  };
}

function parsedUsageEstimate(value: string | null | undefined): ChatTokenEstimateSnapshot | null {
  if (!value) return null;
  try {
    return normalizeUsageEstimate(JSON.parse(value) as Partial<ChatTokenEstimateSnapshot>);
  } catch {
    return null;
  }
}

function parsedUsageAudit(value: string | null | undefined): ChatUsageAuditSnapshot | null {
  if (!value) return null;
  try {
    const parsed = JSON.parse(value) as Partial<ChatUsageAuditSnapshot>;
    const requestEstimate = normalizeUsageEstimate(parsed.requestEstimate);
    if (!requestEstimate) return null;
    return {
      calibrationFactor: typeof parsed.calibrationFactor === "number" ? parsed.calibrationFactor : null,
      cumulativeRunUsage: normalizeTokenUsage(parsed.cumulativeRunUsage),
      inputTokenDelta: typeof parsed.inputTokenDelta === "number" ? parsed.inputTokenDelta : null,
      inputTokenRatio: typeof parsed.inputTokenRatio === "number" ? parsed.inputTokenRatio : null,
      providerUsage: normalizeTokenUsage(parsed.providerUsage),
      requestEstimate,
      responseId: parsed.responseId ?? null,
      usageConfidence: parsed.usageConfidence ?? "unavailable",
      usageSource: parsed.usageSource ?? "unavailable",
    };
  } catch {
    return null;
  }
}

function latestUsageEstimateFromEvents(events: ChatAguiEvent[]) {
  return parsedUsageEstimate(latestEventValue(events, "usage_estimate_json"));
}

function latestUsageAuditFromEvents(events: ChatAguiEvent[]) {
  return parsedUsageAudit(latestEventValue(events, "usage_audit_json"));
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
  requestEstimate: ChatTokenEstimateSnapshot | null | undefined,
  events: ChatAguiEvent[],
) {
  const metadataWindow = tokenFieldFromMetadata(
    contextValue(latestEventValue(events, "contextWindowTokens"))
    ?? contextValue(latestEventValue(events, "context_window_tokens"))
    ?? contextValue(latestEventValue(events, "contextWindowLimit"))
    ?? contextValue(latestEventValue(events, "context_window_limit")),
  );
  return requestEstimate?.contextWindowLimit ?? model?.contextWindowTokens ?? selectedModel?.contextWindowTokens ?? metadataWindow ?? null;
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

function formatEstimateStatus(estimate: ChatTokenEstimateSnapshot | null) {
  if (!estimate) return "unavailable";
  if (estimate.overBudget) return "over budget";
  if (estimate.compactionTriggerReached) return "compaction threshold";
  return estimate.rebased ? "rebased" : "prepared";
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
const liveAguiStartToleranceMs = 5_000;
const activeChatReplayPollIntervalMs = 1_000;
const activeChatReplayPendingTurnMaxAgeMs = 30 * 60_000;

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

function aguiEventThreadId(event: TranscriptAguiEvent) {
  return "threadId" in event && typeof event.threadId === "string" ? event.threadId : "";
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

function latestAguiRunCode(events: TranscriptAguiEvent[]) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const runCode = aguiEventRunCode(events[index]);
    if (runCode) return runCode;
  }
  return "";
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

interface AguiWorkRun {
  events: TranscriptAguiEvent[];
  firstMillis: number;
  runCode: string;
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
    .filter((run) => buildAguiUserWorkCards(run.events).length > 0)
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

function aguiWorkRailsByMessageCode(
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

function mergeTranscriptAguiEvents<T extends TranscriptAguiEvent>(...sources: T[][]) {
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

function isRecentPendingUserTurn(messages: ChatMessage[], now = Date.now()) {
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

function openAguiProtocolRunCodes(events: TranscriptAguiEvent[], sessionCode: string) {
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

function filterTranscriptAguiEventsForSession(
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

function hasToolParameterPayload(value: unknown) {
  if (value == null) return false;
  if (typeof value === "string") return Boolean(value.trim());
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") return true;
  return true;
}

function appendReasoningDelta(content: string, delta: string) {
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

function toolSummaryLabel(toolName: string, args: unknown) {
  const target = valueAtKnownKey(args);
  const title = `${toolActionLabel(toolName, args)} ${toolName}`;
  if (!target || target === toolName) return title;
  return `${title} [${truncateLabel(target, 48)}]`;
}

function isStructuredWorkEvent(event: ChatAguiStreamEvent) {
  const type = (event.type ?? "").toUpperCase();
  return type.startsWith("REASONING") || type.startsWith("TOOL_CALL") || type === "ACTIVITY_DELTA";
}

function terminalRunSummary(event: TranscriptAguiEvent) {
  const type = aguiEventType(event).toUpperCase();
  if (type !== "RUN_ERROR") return "";
  if ("message" in event && typeof event.message === "string" && event.message.trim()) return event.message.trim();
  if ("code" in event && typeof event.code === "string" && event.code.trim()) return event.code.trim();
  return "Run stopped.";
}

function closeRunWorkCards(
  reasoningCards: Map<string, ReasoningWorkCard>,
  activeCardIdsByRun: Map<string, string>,
  toolCards: Map<string, ToolWorkCard>,
  event: TranscriptAguiEvent,
) {
  const runCode = aguiEventRunCode(event);
  if (!runCode) return;
  const endMillis = aguiEventMillis(event) ?? Date.now();
  const summary = terminalRunSummary(event);
  Array.from(reasoningCards.values())
    .filter((card) => card.runCode === runCode && !card.ended)
    .forEach((card) => {
      card.ended = true;
      card.endMillis = endMillis;
      if (summary && !card.content.trim()) card.content = summary;
    });
  Array.from(toolCards.values())
    .filter((card) => card.runCode === runCode && !card.ended)
    .forEach((card) => {
      card.ended = true;
      if (summary && !card.result.trim()) card.result = summary;
    });
  activeCardIdsByRun.delete(runCode);
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
  toolCards: ToolWorkCard[];
}

interface ToolWorkCard {
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

interface ToolGroupWorkCard {
  firstMillis: number;
  id: string;
  runCode: string;
  toolCards: ToolWorkCard[];
}

type AguiUserWorkCard =
  | { kind: "reasoning"; value: ReasoningWorkCard }
  | { kind: "toolGroup"; value: ToolGroupWorkCard }
  | { kind: "tool"; value: ToolWorkCard };

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

function buildAguiUserWorkCards(events: TranscriptAguiEvent[]): AguiUserWorkCard[] {
  const reasoningCards = new Map<string, ReasoningWorkCard>();
  const activeReasoningCardIdsByRun = new Map<string, string>();
  const latestReasoningCardIdsByRun = new Map<string, string>();
  const reasoningSequenceByRun = new Map<string, number>();
  const toolCards = new Map<string, ToolWorkCard>();

  events.forEach((event) => {
    const type = aguiEventType(event).toUpperCase();

    if (type === "REASONING_START") {
      ensureReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event, { forceNew: true });
      return;
    }
    if (type === "REASONING_MESSAGE_START") {
      ensureReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event);
      return;
    }
    if (type === "REASONING_MESSAGE_CONTENT") {
      const card = ensureReasoningCard(reasoningCards, activeReasoningCardIdsByRun, latestReasoningCardIdsByRun, reasoningSequenceByRun, event);
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
      return;
    }
    if (type === "RUN_ERROR" || type === "RUN_FINISHED") {
      closeRunWorkCards(reasoningCards, activeReasoningCardIdsByRun, toolCards, event);
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

interface QueryStatus {
  loading?: boolean;
  error?: Error | null;
  retry?: () => void;
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

function stoppedAguiEvents(sessionCode: string, runCodes: Iterable<string>): ChatAguiStreamEvent[] {
  const stoppedAt = Date.now();
  return Array.from(runCodes)
    .filter((runCode) => runCode && runCode !== "stream")
    .map((runCode, index) => ({
      type: "RUN_ERROR",
      threadId: sessionCode,
      runId: runCode,
      code: "CHAT_STREAM_STOPPED",
      message: "Stopped.",
      timestamp: stoppedAt + index,
    }));
}

function iconButtonClass(active = false) {
  return cn(
    "inline-flex size-8 items-center justify-center rounded-md border border-transparent text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50",
    active && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
  );
}

function smallActionButtonClass() {
  return "inline-flex min-h-8 items-center rounded-md border border-[var(--color-outline-variant)] px-2.5 text-xs font-medium text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50";
}

function QueryStatusLine({
  errorText,
  loadingText,
  retryText,
  status,
}: {
  errorText: string;
  loadingText: string;
  retryText: string;
  status?: QueryStatus;
}) {
  if (status?.loading) {
    return (
      <p className="inline-flex items-center gap-1.5 text-xs text-[var(--color-on-surface-variant)]">
        <LoaderCircle className="animate-spin" size={13} />
        <span>{loadingText}</span>
      </p>
    );
  }
  if (status?.error) {
    return (
      <div className="space-y-2 text-xs text-[var(--color-error)]">
        <div className="flex items-start gap-1.5">
          <CircleAlert className="mt-0.5 shrink-0" size={13} />
          <span>{errorText}</span>
        </div>
        {status.retry ? (
          <button className={smallActionButtonClass()} type="button" onClick={status.retry}>
            <RefreshCw size={13} />
            <span className="ml-1.5">{retryText}</span>
          </button>
        ) : null}
      </div>
    );
  }
  return null;
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
  goal,
  goalBusy,
  goalStatus,
  latestCompaction,
  latestCompactionStatus,
  latestRequestEstimate,
  latestUsageAudit,
  messages,
  model,
  onCancelGoal,
  onCreateCompaction,
  onClearGoal,
  onPauseGoal,
  onResumeGoal,
  onSetGoal,
  compactionBusy,
  selectedModel,
  session,
  todos,
  todosStatus,
}: {
  events: ChatAguiEvent[];
  goal?: ChatSessionGoalView | null;
  goalBusy?: boolean;
  goalStatus?: QueryStatus;
  latestCompaction?: ChatSessionCompaction | null;
  latestCompactionStatus?: QueryStatus;
  latestRequestEstimate?: ChatTokenEstimateSnapshot | null;
  latestUsageAudit?: ChatUsageAuditSnapshot | null;
  messages: ChatMessage[];
  model?: ModelRecord | null;
  onCancelGoal?: () => void;
  onCreateCompaction?: () => void;
  onClearGoal?: () => void;
  onPauseGoal?: () => void;
  onResumeGoal?: () => void;
  onSetGoal?: (objective: string) => Promise<void> | void;
  compactionBusy?: boolean;
  selectedModel?: ChatModelOption | null;
  session?: ChatSession;
  todos?: ChatSessionTodo[];
  todosStatus?: QueryStatus;
}) {
  const { t } = useTranslation("pages");
  const [goalDraft, setGoalDraft] = useState("");
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
  const requestEstimate = latestRequestEstimate ?? latestUsageEstimateFromEvents(events);
  const usageAudit = latestUsageAudit ?? latestUsageAuditFromEvents(events);
  const latestProviderUsage = usageAudit?.providerUsage ?? null;
  const usedTokens = requestEstimate?.estimatedInputTokens ?? null;
  const windowTokens = contextWindowLimit(model, selectedModel, requestEstimate, events);
  const isSystemAgentSession = Boolean(
    session?.agentCode && !isMainChatAgent(session.agentCode),
  );
  const systemAgentSessionId = contextValue(session?.systemAgentSessionId);
  const systemSessionPath = isSystemAgentSession ? systemAgentSessionPath({
    agentCode: session?.agentCode,
    sessionId: systemAgentSessionId,
    worktreePath,
  }) : null;
  const activeCriteria = goal?.criteria.filter((criterion) => criterion.status === "ACTIVE") ?? [];

  async function handleSubmitGoal() {
    const objective = goalDraft.trim();
    if (!objective || !onSetGoal) return;
    await onSetGoal(objective);
    setGoalDraft("");
  }

  return (
    <section className="space-y-4">
      <div className="space-y-5 text-sm">
        <ContextBlock title="Last Request Context">
          <ContextRow label="model" value={modelLabel} />
          <ContextRow label="window" value={formatContextWindowValue(windowTokens, usedTokens)} />
          <ContextRow label="estimated" value={formatTokenCount(requestEstimate?.estimatedInputTokens)} />
          <ContextRow label="messages" value={formatTokenCount(requestEstimate?.messageTokenEstimate)} />
          <ContextRow label="tools" value={formatTokenCount(requestEstimate?.toolSchemaTokenEstimate)} />
          <ContextRow label="budget" value={formatTokenCount(requestEstimate?.effectiveInputBudget)} />
          <ContextRow label="state" value={formatEstimateStatus(requestEstimate)} />
        </ContextBlock>
        <ContextBlock title="Provider Usage">
          <ContextRow label="last turn" value={<TokenUsageCompare lastTurn={lastTurnUsage} sessionTotal={usage} />} />
          <ContextRow label="cached input" value={formatTokenCount(latestProviderUsage?.cachedInputTokens)} />
          <ContextRow label="reasoning" value={formatTokenCount(latestProviderUsage?.reasoningTokens)} />
          <ContextRow label="session spend" value={formatTokenCount(usage.total)} />
          <ContextRow label="input" value={formatTokenCount(usage.input)} />
          <ContextRow label="output" value={formatTokenCount(usage.output)} />
        </ContextBlock>
        <ContextBlock title={t("chat.shortTermMemory.workingState")}>
          <div className="mb-3 space-y-2 border-b border-[var(--color-outline-variant)] pb-3">
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium text-[var(--color-on-surface-variant)]">
                {t("chat.shortTermMemory.goal")}
              </span>
              {goal ? <Badge tone="neutral">{goal.goal.status}</Badge> : null}
            </div>
            {goalStatus?.loading || goalStatus?.error ? (
              <QueryStatusLine
                errorText={t("chat.shortTermMemory.loadFailed")}
                loadingText={t("chat.shortTermMemory.loadingGoal")}
                retryText={t("chat.shortTermMemory.retry")}
                status={goalStatus}
              />
            ) : goal ? (
              <div className="space-y-2">
                <p className="break-words text-sm text-[var(--color-on-surface)]">
                  {goal.goal.objectiveMarkdown}
                </p>
                {activeCriteria.length ? (
                  <ul className="space-y-1 text-xs text-[var(--color-on-surface-variant)]">
                    {activeCriteria.map((criterion) => (
                      <li key={criterion.criterionCode} className="break-words">
                        {criterion.text}
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>
            ) : (
              <p className="text-xs text-[var(--color-on-surface-variant)]">
                {t("chat.shortTermMemory.noGoal")}
              </p>
            )}
          </div>
          {todosStatus?.loading || todosStatus?.error ? (
            <QueryStatusLine
              errorText={t("chat.shortTermMemory.loadFailed")}
              loadingText={t("chat.shortTermMemory.loadingTodos")}
              retryText={t("chat.shortTermMemory.retry")}
              status={todosStatus}
            />
          ) : todos?.length ? (
            <div className="space-y-2">
              {todos.map((todo) => (
                <div key={todo.todoCode} className="grid grid-cols-[1rem_minmax(0,1fr)] items-start gap-2">
                  <ListTodo className="mt-0.5 text-[var(--color-on-surface-variant)]" size={14} />
                  <div className="min-w-0 space-y-1">
                    <div className="break-words text-[var(--color-on-surface)]">{todo.content}</div>
                    <div className="flex flex-wrap gap-1.5">
                      <Badge tone="neutral">{todo.status}</Badge>
                      <Badge tone="neutral">{todo.priority}</Badge>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-xs text-[var(--color-on-surface-variant)]">
              {t("chat.shortTermMemory.emptyTodos")}
            </p>
          )}
        </ContextBlock>
        <ContextBlock title={t("chat.shortTermMemory.goalControls")}>
          {goalStatus?.loading || goalStatus?.error ? (
            <QueryStatusLine
              errorText={t("chat.shortTermMemory.loadFailed")}
              loadingText={t("chat.shortTermMemory.loadingGoal")}
              retryText={t("chat.shortTermMemory.retry")}
              status={goalStatus}
            />
          ) : goal ? (
            <div className="flex flex-wrap gap-1.5">
              <button
                className={smallActionButtonClass()}
                disabled={goalBusy || goal.goal.status === "PAUSED"}
                type="button"
                onClick={onPauseGoal}
              >
                {t("chat.shortTermMemory.pauseGoal")}
              </button>
              <button
                className={smallActionButtonClass()}
                disabled={goalBusy || goal.goal.status === "ACTIVE"}
                type="button"
                onClick={onResumeGoal}
              >
                {t("chat.shortTermMemory.resumeGoal")}
              </button>
              <button
                className={smallActionButtonClass()}
                disabled={goalBusy}
                type="button"
                onClick={onCancelGoal}
              >
                {t("chat.shortTermMemory.cancelGoal")}
              </button>
              <button
                className={smallActionButtonClass()}
                disabled={goalBusy}
                type="button"
                onClick={onClearGoal}
              >
                {t("chat.shortTermMemory.clearGoal")}
              </button>
            </div>
          ) : (
            <div className="space-y-2">
              <textarea
                className="min-h-20 w-full resize-y rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 py-1.5 text-sm text-[var(--color-on-surface)] outline-none transition focus:border-[var(--color-primary)]"
                disabled={!session || goalBusy}
                placeholder={t("chat.shortTermMemory.goalPlaceholder")}
                value={goalDraft}
                onChange={(event) => setGoalDraft(event.target.value)}
              />
              <button
                className={smallActionButtonClass()}
                disabled={!session || goalBusy || !goalDraft.trim()}
                type="button"
                onClick={handleSubmitGoal}
              >
                {goalBusy ? t("chat.shortTermMemory.savingGoal") : t("chat.shortTermMemory.setGoal")}
              </button>
            </div>
          )}
        </ContextBlock>
        <ContextBlock title={t("chat.shortTermMemory.checkpoint")}>
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="text-xs text-[var(--color-on-surface-variant)]">
              {latestCompactionStatus?.loading
                ? t("chat.shortTermMemory.loadingCheckpoint")
                : latestCompactionStatus?.error
                  ? t("chat.shortTermMemory.loadFailed")
                  : latestCompaction
                    ? t("chat.shortTermMemory.checkpointReady")
                    : t("chat.shortTermMemory.noCheckpoint")}
            </span>
            <button
              aria-label={t("chat.shortTermMemory.createCheckpoint")}
              className={iconButtonClass()}
              disabled={!session || compactionBusy}
              title={t("chat.shortTermMemory.createCheckpoint")}
              type="button"
              onClick={onCreateCompaction}
            >
              {compactionBusy ? <LoaderCircle className="animate-spin" size={15} /> : <Archive size={15} />}
            </button>
          </div>
          {latestCompactionStatus?.error ? (
            <QueryStatusLine
              errorText={t("chat.shortTermMemory.loadFailed")}
              loadingText={t("chat.shortTermMemory.loadingCheckpoint")}
              retryText={t("chat.shortTermMemory.retry")}
              status={latestCompactionStatus}
            />
          ) : latestCompaction ? (
            <div className="space-y-1.5">
              <ContextRow label={t("chat.shortTermMemory.rows.checkpoint")} value={latestCompaction.checkpointMessageCode} />
              <ContextRow label={t("chat.shortTermMemory.rows.source")} value={`${latestCompaction.sourceStartSeq}-${latestCompaction.sourceEndSeq}`} />
              <ContextRow label={t("chat.shortTermMemory.rows.created")} value={formatTime(latestCompaction.createdAt)} />
            </div>
          ) : null}
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
  const [expandedModelCode, setExpandedModelCode] = useState<string | null>(null);
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
      setExpandedModelCode(null);
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
                "flex w-full items-start rounded-md px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]",
                model.optionCode === selectedModelCode && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
              )}
              key={model.optionCode}
              type="button"
              role="option"
              title={model.displayName}
              aria-selected={model.optionCode === selectedModelCode}
              onMouseEnter={() => setExpandedModelCode(model.optionCode)}
              onMouseLeave={() => setExpandedModelCode((current) => (
                current === model.optionCode ? null : current
              ))}
              onFocus={() => setExpandedModelCode(model.optionCode)}
              onBlur={() => setExpandedModelCode((current) => (
                current === model.optionCode ? null : current
              ))}
              onClick={() => {
                onSelect(model.optionCode);
                setOpen(false);
              }}
            >
              <span
                className={cn(
                  "block min-w-0 font-medium",
                  expandedModelCode === model.optionCode
                    ? "whitespace-normal break-all overflow-visible"
                    : "truncate",
                )}
              >
                {model.displayName}
              </span>
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
  activeSourceTab,
  collapsed,
  creatingSession,
  groups,
  onCollapseToggle,
  onArchiveSession,
  onCreateSession,
  onDeleteSession,
  onRenameSession,
  onSessionSelect,
  onSourceTabSelect,
  runningSessionCode,
  sessionActionPendingCode,
  unseenSessionCodes,
}: {
  activeSessionCode: string;
  activeSourceTab: ChatSessionSourceTab;
  collapsed: boolean;
  creatingSession: boolean;
  groups: ChatSessionGroup[];
  onCollapseToggle: () => void;
  onArchiveSession: (session: ChatSession) => void;
  onCreateSession: () => void;
  onDeleteSession: (session: ChatSession) => void;
  onRenameSession: (session: ChatSession, title: string) => Promise<void>;
  onSessionSelect: (session: ChatSession, sourceTab?: ChatSessionSourceTab) => void;
  onSourceTabSelect: (sourceTab: ChatSessionSourceTab) => void;
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

  const activeGroups = useMemo(
    () => groups.filter((group) => sourceTabForGroup(group) === activeSourceTab),
    [activeSourceTab, groups],
  );
  const activeFlatSessions = useMemo(
    () => activeGroups.flatMap((group) => group.sessions),
    [activeGroups],
  );
  const sourceTabsBySessionCode = useMemo(() => {
    const items = new Map<string, ChatSessionSourceTab>();
    groups.forEach((group) => {
      const sourceTab = sourceTabForGroup(group);
      group.sessions.forEach((session) => {
        items.set(session.sessionCode, sourceTab);
      });
    });
    return items;
  }, [groups]);

  useEffect(() => () => clearCollapsedTooltipTimer(), []);

  useEffect(() => {
    if (!collapsed) hideCollapsedTooltip();
  }, [collapsed]);

  function renderSessionRow(session: ChatSession) {
    const editing = editingSessionCode === session.sessionCode;
    const pending = sessionActionPendingCode === session.sessionCode;
    const sourceTab = sourceTabsBySessionCode.get(session.sessionCode) ?? activeSourceTab;

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
          <button className="min-w-0 text-left" type="button" onClick={() => onSessionSelect(session, sourceTab)}>
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
  }

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
            {activeFlatSessions.map((session) => (
              <button
                className={iconButtonClass(activeSessionCode === session.sessionCode)}
                key={session.sessionCode}
                type="button"
                aria-label={`Open ${session.title}`}
                onClick={() => onSessionSelect(session, activeSourceTab)}
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
            <div className="mb-4 grid grid-cols-3 gap-1 rounded-md bg-[var(--color-surface-container)] p-1" role="tablist" aria-label="Session source">
              {chatSessionSourceTabs.map((tab) => {
                const TabIcon = tab.icon;
                return (
                  <button
                    className={cn(
                      "flex h-9 min-w-0 items-center justify-center rounded px-2 text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)]",
                      activeSourceTab === tab.id
                        ? "bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] shadow-[0_1px_4px_rgba(15,23,42,0.08)]"
                        : "hover:bg-[var(--color-hover-surface)]",
                    )}
                    key={tab.id}
                    type="button"
                    role="tab"
                    aria-label={tab.label}
                    aria-selected={activeSourceTab === tab.id}
                    title={tab.label}
                    onClick={() => onSourceTabSelect(tab.id)}
                  >
                    <TabIcon size={17} strokeWidth={2.2} />
                    <span className="sr-only">{tab.label}</span>
                  </button>
                );
              })}
            </div>
            {activeSourceTab !== "task" ? (
              <div className="space-y-1">
                {activeFlatSessions.length > 0 ? (
                  activeFlatSessions.map((session) => renderSessionRow(session))
                ) : (
                  <p className="px-2 py-6 text-center text-xs text-[var(--color-on-surface-variant)]">
                    No sessions in this tab.
                  </p>
                )}
              </div>
            ) : activeGroups.map((group) => {
              const groupKey = sessionGroupKey(group);
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
                      {group.sessions.map((session) => renderSessionRow(session))}
                    </div>
                  ) : null}
                </section>
              );
            })}
            {activeSourceTab === "task" && activeGroups.length === 0 ? (
              <p className="px-2 py-6 text-center text-xs text-[var(--color-on-surface-variant)]">
                No sessions in this tab.
              </p>
            ) : null}
          </div>
        </>
      )}
      <DraggleLine onClick={onCollapseToggle} className="absolute right-0 top-0 h-full z-10 " />
    </aside>
  );
}

function ChatBubble({
  currentUserDisplayName,
  message,
  onEditMessage,
}: {
  currentUserDisplayName: string;
  message: ChatMessage;
  onEditMessage?: (content: string) => void;
}) {
  const isUser = message.role === "USER";
  const actorName = isUser ? (currentUserDisplayName || message.actor) : message.actor;
  async function handleEditMessage() {
    if (!isUser || !message.content || !onEditMessage) return;
    onEditMessage(message.content);
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
            <CopyValueButton
              ariaLabel="Copy raw markdown"
              title="Copy raw markdown"
              value={message.content}
            />
          ) : null}
          {isUser ? (
            <>
              <CopyValueButton
                ariaLabel="Copy raw markdown"
                title="Copy raw markdown"
                value={message.content}
              />
              <button
                aria-label="Edit message"
                className="inline-flex size-6 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-40"
                disabled={!message.content || !onEditMessage}
                title="Edit message"
                type="button"
                onClick={() => void handleEditMessage()}
              >
                <Pencil size={14} />
              </button>
            </>
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

function ReasoningUserCard({ card }: { card: ReasoningWorkCard }) {
  const hasContent = card.content.trim().length > 0;
  const detail = hasContent ? card.content : "";
  const initializingHint = !card.ended && !hasContent && card.runCode === "pending"
    ? "System agent 正在初始化..."
    : "";
  const [expanded, setExpanded] = useState(() => !card.ended);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const nestedToolCards = card.toolCards.filter((toolCard) => toolCard.started);
  const hasRunningTool = nestedToolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result);
  const shouldStayExpanded = !card.ended || hasRunningTool;
  const reasoningPreview = hasContent ? deriveReasoningPreview(detail, nestedToolCards) : "";
  const headerSummary = initializingHint || reasoningPreview || (card.ended ? "Reasoning captured." : "Thinking...");

  useEffect(() => {
    if (shouldStayExpanded) {
      setExpanded(true);
    } else {
      setExpanded(false);
    }
  }, [shouldStayExpanded]);

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
    <article
      aria-label="Thinking"
      className="text-sm"
    >
      <button
        className="group flex w-full items-start justify-between gap-3 rounded-md py-1 text-left text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((current) => !current)}
      >
        <div className="flex min-w-0 items-start gap-2">
          <span className="mt-0.5 inline-flex size-5 shrink-0 items-center justify-center rounded text-[var(--color-on-surface-variant)] transition group-hover:bg-[var(--color-surface-container-low)] group-hover:text-[var(--color-on-surface)]">
            <BrainCircuit className={card.ended ? undefined : "animate-pulse"} size={16} />
          </span>
          <span className={cn("min-w-0 flex-1 line-clamp-2 text-sm leading-5", card.ended ? undefined : "animate-pulse")}>
            {headerSummary}
          </span>
        </div>
        <span className="mt-0.5 shrink-0 text-[var(--color-on-surface-variant)]">
          {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        </span>
      </button>
      {expanded ? (
        <div className="mt-1 space-y-3 pb-2 pl-7">
          {detail ? (
            <div
              ref={contentRef}
              className="max-h-36 overflow-y-auto overscroll-contain pr-2"
            >
              <MarkdownContent
                ariaLabel="Streaming reasoning markdown"
                className="min-h-0 py-0 text-sm leading-6 text-[var(--color-on-surface)]"
                content={detail}
              />
            </div>
          ) : null}
        </div>
      ) : null}
    </article>
  );
}

function JsonBlock({ value }: { value: unknown }) {
  const preview = formatJsonBlockValue(value);
  if (!preview) return null;

  return (
    <pre className="overflow-x-auto whitespace-pre-wrap rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-3 font-mono text-xs leading-5 text-[var(--color-on-surface)] [overflow-wrap:anywhere]">
      {preview}
    </pre>
  );
}

function CopyValueButton({
  ariaLabel,
  title,
  value,
}: {
  ariaLabel: string;
  title: string;
  value: string;
}) {
  const [copied, setCopied] = useState(false);
  const resetTimerRef = useRef<number | null>(null);

  useEffect(() => () => {
    if (resetTimerRef.current !== null) window.clearTimeout(resetTimerRef.current);
  }, []);

  async function handleCopy() {
    if (!value.trim()) return;
    try {
      await copyPlainText(value);
      setCopied(true);
      if (resetTimerRef.current !== null) window.clearTimeout(resetTimerRef.current);
      resetTimerRef.current = window.setTimeout(() => setCopied(false), 1400);
    } catch {
      setCopied(false);
    }
  }

  return (
    <button
      aria-label={ariaLabel}
      className="inline-flex size-6 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-40"
      disabled={!value.trim()}
      title={title}
      type="button"
      onClick={() => void handleCopy()}
    >
      {copied ? <CheckCircle2 size={14} /> : <Copy size={14} />}
    </button>
  );
}

function CompactToolRow({
  card,
  nested = false,
  stepNumber,
}: {
  card: ToolWorkCard;
  nested?: boolean;
  stepNumber?: number;
}) {
  const complete = card.ended || Boolean(card.result);
  const hasResultPayload = Boolean(card.result);
  const result = card.result || (complete ? "No result payload." : "");
  const [expanded, setExpanded] = useState(() => !complete);
  const logRef = useRef<HTMLDivElement | null>(null);
  const running = card.started && !complete;
  const hasParameters = hasToolParameterPayload(card.args);
  const parametersText = hasParameters ? formatJsonBlockValue(card.args) : "";
  const resultText = hasResultPayload ? formatJsonBlockValue(card.result) : "";
  const toolLabel = toolSummaryLabel(card.toolName, card.args);
  const nestedToolLabel = `${complete ? "Ran" : "Running"} ${toolLabel.replace(/^Run\s+/i, "")}`;
  const [parametersExpanded, setParametersExpanded] = useState(false);

  useEffect(() => {
    if (running) {
      setExpanded(true);
    } else if (complete) {
      setExpanded(false);
    }
  }, [complete, running]);

  useEffect(() => {
    if (!expanded || complete || !card.logs.length) return;
    const element = logRef.current;
    if (!element) return;
    const frame = window.requestAnimationFrame(() => {
      element.scrollTop = element.scrollHeight;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [card.logs.length, complete, expanded]);

  return (
    <div className={cn(
      nested ? "text-sm" : "rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-2 py-1.5",
    )}>
      <button
        className={cn(
          "flex w-full items-center justify-between gap-3 rounded-md text-left transition",
          nested ? "py-1 text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]" : "px-1 py-1.5 hover:bg-[var(--color-hover-surface)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
        )}
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((current) => !current)}
      >
        <div className="flex min-w-0 items-center gap-2">
          <span className={cn(
            "inline-flex shrink-0 items-center justify-center text-[11px] font-semibold text-[var(--color-on-surface-variant)]",
            nested ? "size-4" : "size-6 rounded-md bg-[var(--color-surface-container-low)]",
          )}>
            {typeof stepNumber === "number"
              ? stepNumber
              : complete
                ? <CheckCircle2 size={14} />
                : <Hammer className="animate-pulse" size={14} />}
          </span>
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <span className={cn("truncate", nested ? "font-normal" : "font-semibold")}>
                {nested ? nestedToolLabel : toolLabel}
              </span>
              {nested ? null : <Badge tone={complete ? "neutral" : "success"}>{complete ? "done" : "running"}</Badge>}
            </div>
          </div>
        </div>
        <span className="mt-1 shrink-0 text-[var(--color-on-surface-variant)]">
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </button>
      {expanded ? (
        <div className={cn(
          "mt-1 border-t border-[var(--color-outline-variant)]",
          nested ? "ml-6 py-2" : "px-1 py-2",
        )}>
          <div className="space-y-2">
            {hasParameters ? (
              <section className="flex min-h-0 flex-col gap-1">
                <div className="flex items-center justify-between gap-2">
                  <button
                    aria-expanded={parametersExpanded}
                    className="inline-flex min-w-0 items-center gap-1 rounded py-0.5 pr-1 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
                    type="button"
                    onClick={() => setParametersExpanded((current) => !current)}
                  >
                    {parametersExpanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                    <span>Parameters</span>
                  </button>
                  <CopyValueButton ariaLabel="Copy parameters" title="Copy parameters" value={parametersText} />
                </div>
                {parametersExpanded ? (
                  <div className="max-h-40 overflow-y-auto">
                    <JsonBlock value={card.args} />
                  </div>
                ) : null}
              </section>
            ) : null}
            {complete ? (
              <section className="flex min-h-0 flex-col gap-1">
                <div className="flex items-center justify-between gap-2">
                  <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Result</h3>
                  <CopyValueButton ariaLabel="Copy result" title="Copy result" value={resultText} />
                </div>
                <div className="max-h-64 overflow-y-auto">
                  {result ? <JsonBlock value={result} /> : (
                    <div className="rounded-md bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                      No result payload.
                    </div>
                  )}
                </div>
              </section>
            ) : (
              <section className="flex min-h-0 flex-col gap-1">
                <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Log</h3>
                <div
                  ref={logRef}
                  className="max-h-64 space-y-1 overflow-y-auto rounded-md bg-[var(--color-surface-container-lowest)] p-3 font-mono text-xs leading-5 text-[var(--color-on-surface)]"
                >
                  {card.logs.length ? card.logs.map((line, index) => (
                    <p className="whitespace-pre-wrap" key={`${card.id}-log-${index}`}>{line}</p>
                  )) : <p className="text-[var(--color-on-surface-variant)]">Waiting for tool activity...</p>}
                </div>
              </section>
            )}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function ToolUserCard({
  card,
  nested = false,
  stepNumber,
}: {
  card: ToolWorkCard;
  nested?: boolean;
  stepNumber?: number;
}) {
  return <CompactToolRow card={card} nested={nested} stepNumber={stepNumber} />;
}

function ToolGroupUserCard({ card }: { card: ToolGroupWorkCard }) {
  const toolCards = card.toolCards.filter((toolCard) => toolCard.started);
  const hasRunningTool = toolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result);
  const toolCount = toolCards.length;
  const [expanded, setExpanded] = useState(() => hasRunningTool);

  useEffect(() => {
    setExpanded(hasRunningTool);
  }, [hasRunningTool]);

  if (!toolCount) return null;

  return (
    <section className="text-sm" aria-label="Tool run steps">
      <button
        aria-expanded={expanded}
        className="group flex w-full items-center justify-between gap-3 rounded-md py-1 text-left text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
        type="button"
        onClick={() => setExpanded((current) => !current)}
      >
        <span className="flex min-w-0 items-center gap-2">
          <span className="inline-flex size-5 shrink-0 items-center justify-center rounded text-[var(--color-on-surface-variant)] transition group-hover:bg-[var(--color-surface-container-low)] group-hover:text-[var(--color-on-surface)]">
            <Terminal className={hasRunningTool ? "animate-pulse" : undefined} size={15} />
          </span>
          <span className="truncate font-medium">
            {`Ran ${toolCount} ${toolCount === 1 ? "command" : "commands"}`}
          </span>
        </span>
        <span className="shrink-0 text-[var(--color-on-surface-variant)]">
          {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        </span>
      </button>
      {expanded ? (
        <ol className="mt-1 space-y-1 pb-2 pl-7">
          {toolCards.map((toolCard, index) => (
            <li key={toolCard.id}>
              <ToolUserCard card={toolCard} nested stepNumber={index + 1} />
            </li>
          ))}
        </ol>
      ) : null}
    </section>
  );
}

function AguiUserWorkRail({ cards }: { cards: AguiUserWorkCard[] }) {
  if (!cards.length) return null;

  return (
    <section className="space-y-2 py-1" aria-label="Agent work rail">
      {cards.map((card) => (
        card.kind === "reasoning"
          ? <ReasoningUserCard card={card.value} key={`reasoning-${card.value.id}`} />
          : card.kind === "toolGroup"
            ? <ToolGroupUserCard card={card.value} key={`tool-group-${card.value.id}`} />
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

function AguiWaitingForModelRail() {
  return (
    <section className="max-w-[920px]" aria-label="Waiting for model">
      <div className="flex w-full items-center gap-3 py-1 text-xs font-medium text-[var(--color-on-surface-variant)]">
        <span className="h-px flex-1 bg-[var(--color-outline-variant)]" />
        <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
          <LoaderCircle className="animate-spin" size={14} />
          Waiting for model...
        </span>
        <span className="h-px flex-1 bg-[var(--color-outline-variant)]" />
      </div>
    </section>
  );
}

function AguiRunWorkRail({
  events,
  preferences,
}: {
  events: TranscriptAguiEvent[];
  preferences: AguiDebugPreferences;
}) {
  const userCards = buildAguiUserWorkCards(events);
  const displayUserCards = userCards;
  const debugEvents = events.filter((event) => shouldShowAguiEventType(aguiEventType(event), preferences));
  const terminalEvent = terminalAguiEvent(events);
  const terminalRunCode = terminalEvent ? aguiEventRunCode(terminalEvent) : "";
  const currentRunCode = latestAguiRunCode(events);
  const collapsedTerminalRunCodeRef = useRef(terminalRunCode);
  const [expanded, setExpanded] = useState(() => !terminalRunCode);

  useEffect(() => {
    if (!currentRunCode) return;
    if (terminalRunCode) {
      if (collapsedTerminalRunCodeRef.current === terminalRunCode) return;
      collapsedTerminalRunCodeRef.current = terminalRunCode;
      setExpanded(false);
      return;
    }
    if (collapsedTerminalRunCodeRef.current === currentRunCode) return;
    collapsedTerminalRunCodeRef.current = "";
    setExpanded(true);
  }, [currentRunCode, terminalRunCode]);

  if (!terminalEvent && !displayUserCards.length && !debugEvents.length) return null;

  return (
    <section className="max-w-[920px] space-y-2" aria-label="AG-UI run work">
      {terminalEvent ? (
        <button
          aria-expanded={expanded}
          className="flex w-full items-center gap-1.5 border-b border-[var(--color-outline-variant)] py-1.5 text-sm font-medium text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
          type="button"
          onClick={() => setExpanded((current) => !current)}
        >
          <span className="whitespace-nowrap">Worked for {aguiWorkDuration(events)}</span>
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </button>
      ) : null}
      {expanded ? (
        <>
          <AguiUserWorkRail cards={displayUserCards} />
          <AguiEventRail events={debugEvents} />
        </>
      ) : null}
    </section>
  );
}

export function ChatManagementPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const routerSearchText = searchParams.toString();
  const [currentSearchText, setCurrentSearchText] = useState(() => (
    typeof window === "undefined"
      ? routerSearchText
      : window.location.search.replace(/^\?/, "")
  ));
  const currentSearchParams = useMemo(() => new URLSearchParams(currentSearchText), [currentSearchText]);
  const requestedSessionCode = currentSearchParams.get("sessionCode")?.trim() ?? "";
  const requestedSourceTab = parseChatSessionSourceTab(currentSearchParams.get("sourceTab"));
  const toast = useToast();
  const { t } = useTranslation("pages");
  const { session: authSession } = useAuthSession();
  const { preferences: aguiDebugPreferences } = useAguiDebugPreferences();
  const { preferences: aguiWorkReplayPreferences } = useAguiWorkReplayPreferences();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const [activeChatDataRefreshKey, setActiveChatDataRefreshKey] = useState(0);
  const [activeChatReplayRefreshKey, setActiveChatReplayRefreshKey] = useState(0);
  const [shortTermMemoryRefreshKey, setShortTermMemoryRefreshKey] = useState(0);
  const groups = useChatSessionGroupList(activeWorkspaceCode, activeChatDataRefreshKey);
  const [sessionGroups, setSessionGroups] = useState<ChatSessionGroup[]>([]);
  const [activeSessionCode, setActiveSessionCode] = useState("");
  const [activeSourceTab, setActiveSourceTab] = useState<ChatSessionSourceTab>(() => (
    requestedSourceTab ?? defaultChatSessionSourceTab
  ));
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [composer, setComposer] = useState("");
  const [optimisticActiveSession, setOptimisticActiveSession] = useState<ChatSession | null>(null);
  const requestedSession = requestedSessionCode ? findSession(sessionGroups, requestedSessionCode) : undefined;
  const optimisticSession = optimisticActiveSession
    && optimisticActiveSession.sessionCode === (requestedSessionCode || activeSessionCode)
    ? optimisticActiveSession
    : undefined;
  const resolvedActiveSessionCode = requestedSession?.sessionCode ?? optimisticSession?.sessionCode ?? activeSessionCode;
  const selectedSession = requestedSession
    ?? optimisticSession
    ?? (!requestedSessionCode && resolvedActiveSessionCode ? findSession(sessionGroups, resolvedActiveSessionCode) : undefined);
  const activeSession = selectedSession ?? (
    !requestedSessionCode && !requestedSourceTab && !activeSessionCode
      ? firstSessionForSourceTab(sessionGroups, defaultChatSessionSourceTab)
      : undefined
  );
  const messages = useChatMessageListBySession(activeSession?.sessionCode ?? "", activeChatDataRefreshKey);
  const shortTermMemoryTodosState = useChatSessionTodosState(activeSession?.sessionCode ?? "", shortTermMemoryRefreshKey);
  const latestCompactionState = useChatLatestCompactionState(activeSession?.sessionCode ?? "", shortTermMemoryRefreshKey);
  const activeGoalState = useChatSessionGoalState(activeSession?.sessionCode ?? "", shortTermMemoryRefreshKey);
  const shortTermMemoryTodos = shortTermMemoryTodosState.data;
  const latestCompaction = latestCompactionState.data;
  const activeGoal = activeGoalState.data;
  const shortTermMemoryTodosStatus = useMemo<QueryStatus>(() => ({
    loading: shortTermMemoryTodosState.loading,
    error: shortTermMemoryTodosState.error,
    retry: shortTermMemoryTodosState.retry,
  }), [shortTermMemoryTodosState.error, shortTermMemoryTodosState.loading, shortTermMemoryTodosState.retry]);
  const latestCompactionStatus = useMemo<QueryStatus>(() => ({
    loading: latestCompactionState.loading,
    error: latestCompactionState.error,
    retry: latestCompactionState.retry,
  }), [latestCompactionState.error, latestCompactionState.loading, latestCompactionState.retry]);
  const goalStatus = useMemo<QueryStatus>(() => ({
    loading: activeGoalState.loading,
    error: activeGoalState.error,
    retry: activeGoalState.retry,
  }), [activeGoalState.error, activeGoalState.loading, activeGoalState.retry]);
  const aguiEvents = useChatAguiEvents(activeSession?.sessionCode ?? "");
  const aguiProtocolEvents = useChatAguiProtocolEvents(activeSession?.sessionCode ?? "", activeChatReplayRefreshKey);
  const [visibleMessages, setVisibleMessages] = useState<ChatMessage[]>([]);
  const [liveAguiEventsBySessionCode, setLiveAguiEventsBySessionCode] = useState<Record<string, ChatAguiStreamEvent[]>>({});
  const [liveAguiAnchorMessageCodesBySessionCode, setLiveAguiAnchorMessageCodesBySessionCode] = useState<Record<string, Record<string, string>>>({});
  const [latestRequestEstimatesBySessionCode, setLatestRequestEstimatesBySessionCode] = useState<Record<string, ChatTokenEstimateSnapshot>>({});
  const [latestUsageAuditsBySessionCode, setLatestUsageAuditsBySessionCode] = useState<Record<string, ChatUsageAuditSnapshot>>({});
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
  const [compactingSessionCode, setCompactingSessionCode] = useState("");
  const [goalActionSessionCode, setGoalActionSessionCode] = useState("");
  const [sessionActionPendingCode, setSessionActionPendingCode] = useState("");
  const [deleteTargetSession, setDeleteTargetSession] = useState<ChatSession | null>(null);
  const [unseenSessionCodes, setUnseenSessionCodes] = useState<Set<string>>(() => new Set());
  const [showScrollToBottom, setShowScrollToBottom] = useState(false);
  const [inspectionAttachments, setInspectionAttachments] = useState<ChatMessageAttachment[]>([]);
  const activeSessionCodeRef = useRef("");
  const modelSelectionSessionRef = useRef("");
  const streamAbortControllerRef = useRef<AbortController | null>(null);
  const streamAbortRequestedRef = useRef(false);
  const requestedSessionCodeSyncedRef = useRef("");
  const autoStickToBottomRef = useRef(true);
  const messageScrollRef = useRef<HTMLElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const scrollFrameRef = useRef<number | null>(null);
  const visibleSessionCodeRef = useRef("");
  const liveAssistantDraftRef = useRef("");
  const liveAguiObservedRef = useRef(false);
  const currentLiveAguiRunCodesRef = useRef<Set<string>>(new Set());
  const selectedAgent = chatAgents.find((agent) => agent.agentCode === selectedAgentCode) ?? null;
  const agentScopedModels = useMemo(
    () => modelsForAgent(chatModels, selectedAgent),
    [chatModels, selectedAgent],
  );
  const selectedModel = agentScopedModels.find((model) => model.optionCode === selectedModelCode) ?? null;
  const selectedModelAgentCode = inferChatAgentCodeFromModel(selectedModel, selectedAgentCode || mainChatAgentOption.agentCode);
  const effectiveAgentCode = selectedModelAgentCode || selectedAgentCode || mainChatAgentOption.agentCode;
  const selectedMainAgentModel = selectedModel?.modelCode
    ? chatModels.find((model) => model.modelCode === selectedModel.modelCode) ?? null
    : null;
  const hasActiveSession = Boolean(activeSession);
  const sessionStarted = Boolean(activeSession?.lastRunCode || visibleMessages.length > 0);
  const agentSelectionLocked = Boolean(activeSession && sessionStarted);
  const displayedAgentCode = sessionStarted
    ? (activeSession?.agentCode ?? selectedModelAgentCode ?? selectedAgentCode ?? mainChatAgentOption.agentCode)
    : (selectedModelAgentCode || selectedAgentCode || activeSession?.agentCode || mainChatAgentOption.agentCode);
  const modelEmptyLabel = isMainChatAgent(effectiveAgentCode)
    ? "No text model"
    : `No models for ${selectedAgent?.name ?? effectiveAgentCode}`;
  const currentUserDisplayName = authSession?.user.displayName ?? "";
  const { createInspectionAttachment } = useInspection()

  const replaceChatSearchParams = useCallback((update: (next: URLSearchParams) => void) => {
    const currentSearch = typeof window === "undefined"
      ? currentSearchText
      : window.location.search.replace(/^\?/, "");
    const next = new URLSearchParams(currentSearch);
    update(next);
    const nextSearch = next.toString();
    if (nextSearch === currentSearch) {
      if (currentSearchText !== nextSearch) setCurrentSearchText(nextSearch);
      return;
    }
    setCurrentSearchText(nextSearch);
    const pathname = typeof window === "undefined" ? "/chats" : window.location.pathname;
    navigate(`${pathname}${nextSearch ? `?${nextSearch}` : ""}`, { replace: true });
  }, [currentSearchText, navigate]);

  useEffect(() => {
    setCurrentSearchText(routerSearchText);
  }, [routerSearchText]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const syncCurrentSearch = () => setCurrentSearchText(window.location.search.replace(/^\?/, ""));
    window.addEventListener("popstate", syncCurrentSearch);
    return () => window.removeEventListener("popstate", syncCurrentSearch);
  }, []);

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

  function selectSourceTab(sourceTab: ChatSessionSourceTab) {
    setActiveSourceTab(sourceTab);
    writeStoredChatSessionSourceTab(sourceTab);
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
    if (!optimisticActiveSession) return;
    if (requestedSession?.sessionCode === optimisticActiveSession.sessionCode) {
      setOptimisticActiveSession(null);
    }
  }, [optimisticActiveSession, requestedSession?.sessionCode]);

  useEffect(() => {
    if (!requestedSessionCode) {
      requestedSessionCodeSyncedRef.current = "";
      return;
    }
    if (requestedSessionCodeSyncedRef.current === requestedSessionCode && activeSessionCode === requestedSessionCode) return;
    if (!findSession(sessionGroups, requestedSessionCode)) return;
    if (activeSessionCodeRef.current !== requestedSessionCode) setVisibleMessages([]);
    activeSessionCodeRef.current = requestedSessionCode;
    setActiveSessionCode(requestedSessionCode);
    const sourceTab = sourceTabForSessionCode(sessionGroups, requestedSessionCode);
    if (sourceTab) selectSourceTab(sourceTab);
    requestedSessionCodeSyncedRef.current = requestedSessionCode;
  }, [activeSessionCode, requestedSessionCode, sessionGroups]);

  useEffect(() => {
    if (!requestedSourceTab) return;
    selectSourceTab(requestedSourceTab);
    if (!sessionGroups.length) return;

    if (requestedSessionCode) {
      const requestedSessionSourceTab = sourceTabForSessionCode(sessionGroups, requestedSessionCode);
      if (requestedSessionSourceTab && requestedSessionSourceTab !== requestedSourceTab) {
        replaceChatSearchParams((next) => {
          next.set("sourceTab", requestedSessionSourceTab);
        });
      }
      return;
    }

    const nextSession = firstSessionForSourceTab(sessionGroups, requestedSourceTab);
    if (nextSession) {
      if (activeSessionCodeRef.current !== nextSession.sessionCode) setVisibleMessages([]);
      activeSessionCodeRef.current = nextSession.sessionCode;
      setActiveSessionCode(nextSession.sessionCode);
      replaceChatSearchParams((next) => {
        next.set("sessionCode", nextSession.sessionCode);
        next.set("sourceTab", requestedSourceTab);
      });
      return;
    }

    if (!requestedSessionCode && !activeSessionCodeRef.current) return;
    activeSessionCodeRef.current = "";
    setActiveSessionCode("");
    setVisibleMessages([]);
    replaceChatSearchParams((next) => {
      next.delete("sessionCode");
      next.set("sourceTab", requestedSourceTab);
    });
  }, [replaceChatSearchParams, requestedSessionCode, requestedSourceTab, sessionGroups]);

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
    if (requestedSessionCode || requestedSourceTab || activeSessionCode) return;
    const defaultSession = firstSessionForSourceTab(sessionGroups, defaultChatSessionSourceTab);
    if (defaultSession) {
      setActiveSessionCode(defaultSession.sessionCode);
      selectSourceTab(defaultChatSessionSourceTab);
    }
  }, [activeSessionCode, requestedSessionCode, requestedSourceTab, sessionGroups]);

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
    if (!activeSession?.sessionCode || editingTitle || deleteTargetSession || !shouldAutoFocusChatComposer()) {
      return;
    }
    const composerElement = composerRef.current;
    if (!composerElement) return;
    const focusId = window.requestAnimationFrame(() => {
      composerElement.focus();
    });
    return () => {
      window.cancelAnimationFrame(focusId);
    };
  }, [activeSession?.sessionCode, deleteTargetSession, editingTitle]);

  useEffect(() => {
    const sessionCode = activeSession?.sessionCode ?? "";
    const nextMessages = sessionCode ? messages.filter((message) => message.sessionCode === sessionCode) : [];
    setVisibleMessages((current) => {
      if (
        current.length === nextMessages.length
        && current.every((message, index) => message === nextMessages[index])
      ) {
        return current;
      }
      return nextMessages;
    });
    if (visibleSessionCodeRef.current !== sessionCode) {
      visibleSessionCodeRef.current = sessionCode;
    }
  }, [activeSession?.sessionCode, messages]);

  const liveAguiEvents = activeSession?.sessionCode
    ? liveAguiEventsBySessionCode[activeSession.sessionCode] ?? emptyLiveAguiEvents
    : emptyLiveAguiEvents;
  const liveAguiAnchorMessageCodes = activeSession?.sessionCode
    ? liveAguiAnchorMessageCodesBySessionCode[activeSession.sessionCode] ?? emptyLiveAguiAnchorMessageCodes
    : emptyLiveAguiAnchorMessageCodes;
  const liveAguiPending = Boolean(
    sending
    && activeSession
    && runningSessionCode === activeSession.sessionCode
    && !liveAguiEvents.length,
  );
  const transcriptAguiEvents = useMemo<TranscriptAguiEvent[]>(() => {
    if (liveAguiPending && !aguiProtocolEvents.length) return [];
    return filterTranscriptAguiEventsForSession(
      mergeTranscriptAguiEvents(aguiProtocolEvents, liveAguiEvents),
      activeSession?.sessionCode ?? "",
      visibleMessages,
      liveAguiAnchorMessageCodes,
    );
  }, [
    activeSession?.sessionCode,
    aguiProtocolEvents,
    liveAguiAnchorMessageCodes,
    liveAguiEvents,
    liveAguiPending,
    visibleMessages,
  ]);
  const waitingForModelAnchorMessageCode = useMemo(() => {
    if (!liveAguiPending || !activeSession?.sessionCode) return "";
    for (let index = visibleMessages.length - 1; index >= 0; index -= 1) {
      const message = visibleMessages[index];
      if (message.sessionCode === activeSession.sessionCode && message.role === "USER") {
        return message.messageCode;
      }
    }
    return "";
  }, [activeSession?.sessionCode, liveAguiPending, visibleMessages]);
  const workRailsByMessageCode = useMemo(() => (
    aguiWorkRailsByMessageCode(
      visibleMessages,
      transcriptAguiEvents,
      liveAguiAnchorMessageCodes,
      aguiWorkReplayPreferences.recentTurnLimit,
    )
  ), [aguiWorkReplayPreferences.recentTurnLimit, liveAguiAnchorMessageCodes, transcriptAguiEvents, visibleMessages]);
  const canAbortCurrentSession = Boolean(
    sending
    && activeSession
    && runningSessionCode === activeSession.sessionCode,
  );
  const hasOpenReplayRun = useMemo(() => (
    Boolean(activeSession?.sessionCode)
    && openAguiProtocolRunCodes(aguiProtocolEvents, activeSession?.sessionCode ?? "").size > 0
  ), [activeSession?.sessionCode, aguiProtocolEvents]);
  const shouldPollActiveChatData = Boolean(
    activeSession?.sessionCode
    && !(sending && runningSessionCode === activeSession.sessionCode)
    && (isRecentPendingUserTurn(visibleMessages) || hasOpenReplayRun)
  );
  const shouldPollActiveChatReplay = Boolean(
    activeSession?.sessionCode
    && (
      (sending && runningSessionCode === activeSession.sessionCode)
      || shouldPollActiveChatData
    )
  );
  const handleCancelGoalForRightPanel = useStableCallback(handleCancelGoal);
  const handleClearGoalForRightPanel = useStableCallback(handleClearGoal);
  const handleCreateCompactionForRightPanel = useStableCallback(handleCreateCompaction);
  const handlePauseGoalForRightPanel = useStableCallback(handlePauseGoal);
  const handleResumeGoalForRightPanel = useStableCallback(handleResumeGoal);
  const handleSendForRightPanel = useStableCallback(handleSend);
  const handleSetGoalForRightPanel = useStableCallback(handleSetGoal);

  const rightPanel = useMemo(() => {
    if (!hasActiveSession) return undefined;
    return (
      <Tabs defaultValue="1" className="w-full h-full">
        <TabsList className="mx-4 my-2">
          <TabsTrigger value="1">Run Context</TabsTrigger>
          <TabsTrigger value="2">页面</TabsTrigger>
        </TabsList>
        <TabsContent value="1">
          <div className="m-4">
            <RightContextSidebar
              events={aguiEvents}
              goal={activeGoal}
              goalBusy={Boolean(activeSession?.sessionCode && goalActionSessionCode === activeSession.sessionCode)}
              goalStatus={goalStatus}
              latestCompaction={latestCompaction}
              latestCompactionStatus={latestCompactionStatus}
              latestRequestEstimate={activeSession ? latestRequestEstimatesBySessionCode[activeSession.sessionCode] ?? null : null}
              latestUsageAudit={activeSession ? latestUsageAuditsBySessionCode[activeSession.sessionCode] ?? null : null}
              messages={visibleMessages}
              model={selectedMainAgentModel}
              onCancelGoal={handleCancelGoalForRightPanel}
              compactionBusy={Boolean(activeSession?.sessionCode && compactingSessionCode === activeSession.sessionCode)}
              onCreateCompaction={handleCreateCompactionForRightPanel}
              onClearGoal={handleClearGoalForRightPanel}
              onPauseGoal={handlePauseGoalForRightPanel}
              onResumeGoal={handleResumeGoalForRightPanel}
              onSetGoal={handleSetGoalForRightPanel}
              selectedModel={selectedModel}
              session={activeSession}
              todos={shortTermMemoryTodos}
              todosStatus={shortTermMemoryTodosStatus}
            />
          </div>
        </TabsContent>
        <TabsContent value="2">
          <PageView
            url="https://www.baidu.com"
            onInspectionConfirm={(result) => {
              if (inspectionSubmitMode(result) === "send") {
                void handleSendForRightPanel(inspectionMessageContent(result), [createInspectionAttachment(result)]);
                return;
              }

              setInspectionAttachments((current) => [
                ...current,
                createInspectionAttachment(result),
              ]);
            }}
          />
        </TabsContent>
      </Tabs>
    );
  }, [
    activeSession,
    activeGoal,
    aguiEvents,
    compactingSessionCode,
    createInspectionAttachment,
    goalActionSessionCode,
    goalStatus,
    hasActiveSession,
    handleCancelGoalForRightPanel,
    handleClearGoalForRightPanel,
    handleCreateCompactionForRightPanel,
    handlePauseGoalForRightPanel,
    handleResumeGoalForRightPanel,
    handleSendForRightPanel,
    handleSetGoalForRightPanel,
    latestCompaction,
    latestCompactionStatus,
    latestRequestEstimatesBySessionCode,
    latestUsageAuditsBySessionCode,
    selectedMainAgentModel,
    selectedModel,
    shortTermMemoryTodos,
    shortTermMemoryTodosStatus,
    visibleMessages,
  ]);
  const transcriptDebugOptions = useMemo(() => ({
    debugEventTypes: resolveAguiDebugEventTypes(aguiDebugPreferences),
    recentTurnLimit: aguiWorkReplayPreferences.recentTurnLimit,
  }), [aguiDebugPreferences, aguiWorkReplayPreferences.recentTurnLimit]);
  const transcriptFooterAddon = useMemo(() => (
    <div className="flex min-w-0 max-w-full flex-col items-start gap-2">
      {inspectionAttachments.length ? (
        <div className="flex max-w-full flex-wrap gap-2">
          {inspectionAttachments.map((attachment) => (
            <ChatInspectionAttachmentChip
              attachment={attachment}
              key={attachment.id}
              onRemove={() => {
                setInspectionAttachments((current) => current.filter((item) => item.id !== attachment.id));
              }}
            />
          ))}
        </div>
      ) : null}
      <div className="flex min-w-0 max-w-full items-center gap-1">
        {agentSelectionLocked ? null : (
          <div className="min-w-0 max-w-[9.5rem] shrink">
            <ChatAgentSelector
              locked={agentSelectionLocked}
              loading={agentsLoading}
              agents={chatAgents}
              selectedAgentCode={selectedAgentCode}
              onSelect={handleAgentSelect}
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
    </div>
  ), [
    agentScopedModels,
    agentSelectionLocked,
    agentsLoading,
    chatAgents,
    inspectionAttachments,
    modelEmptyLabel,
    modelsLoading,
    selectedAgentCode,
    selectedModelCode,
  ]);

  useConsoleShellConfig({ rightPanel });

  useEffect(() => {
    if (!shouldPollActiveChatReplay) return;
    const intervalId = window.setInterval(() => {
      setActiveChatReplayRefreshKey((current) => current + 1);
    }, activeChatReplayPollIntervalMs);
    return () => window.clearInterval(intervalId);
  }, [activeSession?.sessionCode, shouldPollActiveChatReplay]);

  useEffect(() => {
    if (!shouldPollActiveChatData) return;
    const intervalId = window.setInterval(() => {
      setActiveChatDataRefreshKey((current) => current + 1);
    }, activeChatReplayPollIntervalMs);
    return () => window.clearInterval(intervalId);
  }, [activeSession?.sessionCode, shouldPollActiveChatData]);

  useEffect(() => {
    if (autoStickToBottomRef.current) scrollToLatestMessage("auto");
    else handleTranscriptScroll();
  }, [transcriptAguiEvents, visibleMessages]);

  function handleSessionSelect(sessionOrCode: ChatSession | string, sourceTab?: ChatSessionSourceTab) {
    const sessionCode = typeof sessionOrCode === "string" ? sessionOrCode : sessionOrCode.sessionCode;
    const selected = typeof sessionOrCode === "string" ? findSession(sessionGroups, sessionCode) : sessionOrCode;
    if (sessionCode !== activeSessionCode) setVisibleMessages([]);
    setOptimisticActiveSession(selected ?? null);
    activeSessionCodeRef.current = sessionCode;
    setActiveSessionCode(sessionCode);
    const nextSourceTab = sourceTab ?? sourceTabForSessionCode(sessionGroups, sessionCode);
    if (nextSourceTab) selectSourceTab(nextSourceTab);
    replaceChatSearchParams((next) => {
      if (sessionCode) {
        next.set("sessionCode", sessionCode);
        if (nextSourceTab) next.set("sourceTab", nextSourceTab);
      } else {
        next.delete("sessionCode");
        next.delete("sourceTab");
      }
    });
    setUnseenSessionCodes((current) => {
      if (!current.has(sessionCode)) return current;
      const next = new Set(current);
      next.delete(sessionCode);
      return next;
    });
  }

  function handleSourceTabSelect(sourceTab: ChatSessionSourceTab) {
    selectSourceTab(sourceTab);
    const nextSession = firstSessionForSourceTab(sessionGroups, sourceTab);
    if (nextSession) {
      handleSessionSelect(nextSession, sourceTab);
      return;
    }
    activeSessionCodeRef.current = "";
    setOptimisticActiveSession(null);
    setActiveSessionCode("");
    setVisibleMessages([]);
    replaceChatSearchParams((next) => {
      next.delete("sessionCode");
      next.set("sourceTab", sourceTab);
    });
  }

  function selectSessionAfterRemoval(groupsAfterRemoval: ChatSessionGroup[], sourceTab: ChatSessionSourceTab) {
    selectSourceTab(sourceTab);
    const nextSession = firstSessionForSourceTab(groupsAfterRemoval, sourceTab);
    if (nextSession) {
      handleSessionSelect(nextSession, sourceTab);
      return;
    }

    activeSessionCodeRef.current = "";
    setOptimisticActiveSession(null);
    setActiveSessionCode("");
    setVisibleMessages([]);
    replaceChatSearchParams((next) => {
      next.delete("sessionCode");
      next.set("sourceTab", sourceTab);
    });
  }

  function handleAgentSelect(agentCode: string) {
    setSelectedAgentCode(agentCode);
    if (!activeSession) return;
    setSessionGroups((current) => replaceSession(current, {
      ...activeSession,
      agentCode,
    }));
  }

  function handleModelSelect(optionCode: string) {
    setSelectedModelCode(optionCode);
    const selectedOption = agentScopedModels.find((model) => model.optionCode === optionCode) ?? null;
    const nextAgentCode = inferChatAgentCodeFromModel(selectedOption, selectedAgentCode || mainChatAgentOption.agentCode);
    setSelectedAgentCode((current) => (current === nextAgentCode ? current : nextAgentCode));
    if (activeSession) {
      setSessionGroups((current) => replaceSession(current, {
        ...activeSession,
        agentCode: nextAgentCode,
      }));
    }
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

  async function handleSend(contentOverride?: string, attachmentOverride?: ChatMessageAttachment[]) {
    if (sending) return;
    if (!activeSession) return;
    const content = (contentOverride ?? composer).trim();
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
    const pendingInspectionAttachments = attachmentOverride ?? inspectionAttachments;
    if (pendingInspectionAttachments.length) {
      sendInput.attachments = pendingInspectionAttachments;
    }
    if (selectedModel.source === "system-agent") {
      sendInput.systemModelId = selectedModel.modelName;
    } else {
      sendInput.modelCode = selectedModel.modelCode;
    }
    const sendingSessionCode = activeSession.sessionCode;
    const abortController = new AbortController();
    streamAbortRequestedRef.current = false;
    streamAbortControllerRef.current = abortController;
    currentLiveAguiRunCodesRef.current.clear();
    const sendStartedAt = Date.now();
    const streamAcceptedAfter = sendStartedAt - liveAguiStartToleranceMs;
    const optimisticUserCode = `LOCAL-${sendStartedAt}-USER`;
    const optimisticAssistantCode = `LOCAL-${sendStartedAt}-ASSISTANT`;
    const optimisticCreatedAt = new Date().toISOString();
    const optimisticUserMessage: ChatMessage = {
      actor: "User",
      channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content,
      attachments: pendingInspectionAttachments,
      createdAt: optimisticCreatedAt,
      id: -sendStartedAt,
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
      id: -sendStartedAt - 1,
      messageCode: optimisticAssistantCode,
      role: "ASSISTANT",
      runCode: null,
      sessionCode: sendingSessionCode,
    };
    setSending(true);
    setRunningSessionCode(sendingSessionCode);
    if (contentOverride === undefined) {
      setComposer("");
    }
    if (attachmentOverride === undefined) {
      setInspectionAttachments([]);
    }
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
            const eventMillis = aguiEventMillis(event);
            if (eventMillis !== null && eventMillis < streamAcceptedAfter) return;
            const runCode = aguiEventRunCode(event);
            if (runCode && runCode !== "stream") {
              currentLiveAguiRunCodesRef.current.add(runCode);
              setLiveAguiAnchorMessageCodesBySessionCode((current) => {
                const sessionAnchors = current[sendingSessionCode] ?? {};
                if (sessionAnchors[runCode]) return current;
                return {
                  ...current,
                  [sendingSessionCode]: {
                    ...sessionAnchors,
                    [runCode]: optimisticUserCode,
                  },
                };
              });
            }
            if (isStructuredWorkEvent(event)) {
              liveAguiObservedRef.current = true;
              if (activeSessionCodeRef.current === sendingSessionCode) {
                setVisibleMessages((current) => current.filter((message) => (
                  message.messageCode !== optimisticAssistantCode || Boolean(message.content.trim())
                )));
              }
            }
            setLiveAguiEventsBySessionCode((current) => ({
              ...current,
              [sendingSessionCode]: mergeTranscriptAguiEvents(current[sendingSessionCode] ?? [], [event]),
            }));
          },
          onDelta: (delta) => {
            liveAssistantDraftRef.current = `${liveAssistantDraftRef.current}${delta}`;
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
      setLatestRequestEstimatesBySessionCode((current) => {
        if (result.run.latestRequestEstimate) {
          return {
            ...current,
            [result.session.sessionCode]: result.run.latestRequestEstimate,
          };
        }
        if (!(result.session.sessionCode in current)) return current;
        const next = { ...current };
        delete next[result.session.sessionCode];
        return next;
      });
      setLatestUsageAuditsBySessionCode((current) => {
        if (result.run.latestUsageAudit) {
          return {
            ...current,
            [result.session.sessionCode]: result.run.latestUsageAudit,
          };
        }
        if (!(result.session.sessionCode in current)) return current;
        const next = { ...current };
        delete next[result.session.sessionCode];
        return next;
      });
      const resultUserMessage = result.messages.find((message) => message.role === "USER") ?? null;
      const persistedUserCode = resultUserMessage?.messageCode ?? optimisticUserCode;
      const runCodes = Array.from(currentLiveAguiRunCodesRef.current);
      if (runCodes.length) {
        setLiveAguiAnchorMessageCodesBySessionCode((current) => {
          const sessionAnchors = current[result.session.sessionCode] ?? {};
          const nextSessionAnchors = { ...sessionAnchors };
          runCodes.forEach((runCode) => {
            nextSessionAnchors[runCode] = persistedUserCode;
          });
          return {
            ...current,
            [result.session.sessionCode]: nextSessionAnchors,
          };
        });
      }
      if (activeSessionCodeRef.current === result.session.sessionCode) {
        setVisibleMessages((current) => [
          ...current.filter((message) => (
            message.messageCode !== optimisticUserCode
            && message.messageCode !== optimisticAssistantCode
          )),
          ...result.messages,
        ]);
      }
      setShortTermMemoryRefreshKey((current) => current + 1);
      setUnseenSessionCodes((current) => {
        const next = new Set(current);
        if (activeSessionCodeRef.current === result.session.sessionCode) next.delete(result.session.sessionCode);
        else next.add(result.session.sessionCode);
        return next;
      });
    } catch (error) {
      const stopped = streamAbortRequestedRef.current || isAbortError(error);
      if (stopped) {
        const terminalEvents = stoppedAguiEvents(sendingSessionCode, currentLiveAguiRunCodesRef.current);
        if (terminalEvents.length) {
          setLiveAguiEventsBySessionCode((current) => ({
            ...current,
            [sendingSessionCode]: mergeTranscriptAguiEvents(current[sendingSessionCode] ?? [], terminalEvents),
          }));
        }
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
      if (pendingInspectionAttachments.length) {
        setInspectionAttachments((current) => [...pendingInspectionAttachments, ...current]);
      }
      if (activeSessionCodeRef.current === sendingSessionCode) {
        setVisibleMessages((current) => current.filter((item) => (
          item.messageCode !== optimisticAssistantCode || Boolean(item.content.trim())
        )));
      }
      toast.error(toastMessage, { title: "Chat send failed" });
    } finally {
      setSending(false);
      setRunningSessionCode("");
      liveAssistantDraftRef.current = "";
      liveAguiObservedRef.current = false;
      if (streamAbortControllerRef.current === abortController) {
        streamAbortControllerRef.current = null;
      }
      streamAbortRequestedRef.current = false;
    }
  }

  async function handleCreateCompaction() {
    if (!activeSession || compactingSessionCode) return;
    const sessionCode = activeSession.sessionCode;
    setCompactingSessionCode(sessionCode);
    try {
      await createChatCompaction({
        sessionCode,
        reason: "manual",
      });
      setShortTermMemoryRefreshKey((current) => current + 1);
      toast.success(t("chat.shortTermMemory.messages.created"), {
        title: t("chat.shortTermMemory.messages.createdTitle"),
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.shortTermMemory.messages.failed"), {
        title: t("chat.shortTermMemory.messages.failedTitle"),
      });
    } finally {
      setCompactingSessionCode("");
    }
  }

  async function runGoalAction(action: () => Promise<unknown>, successMessage: string) {
    if (!activeSession || goalActionSessionCode) return;
    const sessionCode = activeSession.sessionCode;
    setGoalActionSessionCode(sessionCode);
    try {
      await action();
      setShortTermMemoryRefreshKey((current) => current + 1);
      toast.success(successMessage, {
        title: t("chat.shortTermMemory.messages.goalUpdatedTitle"),
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.shortTermMemory.messages.goalFailed"), {
        title: t("chat.shortTermMemory.messages.goalFailedTitle"),
      });
    } finally {
      setGoalActionSessionCode("");
    }
  }

  async function handleSetGoal(objective: string) {
    if (!activeSession) return;
    await runGoalAction(
      () => setChatSessionGoal({
        sessionCode: activeSession.sessionCode,
        objective,
        maxRuns: 20,
        criteria: [],
      }),
      t("chat.shortTermMemory.messages.goalSet"),
    );
  }

  async function handlePauseGoal() {
    if (!activeSession || !activeGoal) return;
    await runGoalAction(
      () => pauseChatSessionGoal({
        sessionCode: activeSession.sessionCode,
        goalCode: activeGoal.goal.goalCode,
      }),
      t("chat.shortTermMemory.messages.goalPaused"),
    );
  }

  async function handleResumeGoal() {
    if (!activeSession || !activeGoal) return;
    await runGoalAction(
      () => resumeChatSessionGoal({
        sessionCode: activeSession.sessionCode,
        goalCode: activeGoal.goal.goalCode,
      }),
      t("chat.shortTermMemory.messages.goalResumed"),
    );
  }

  async function handleCancelGoal() {
    if (!activeSession || !activeGoal) return;
    await runGoalAction(
      () => cancelChatSessionGoal({
        sessionCode: activeSession.sessionCode,
        goalCode: activeGoal.goal.goalCode,
      }),
      t("chat.shortTermMemory.messages.goalCancelled"),
    );
  }

  async function handleClearGoal() {
    if (!activeSession || !activeGoal) return;
    await runGoalAction(
      () => clearChatSessionGoal({
        sessionCode: activeSession.sessionCode,
        goalCode: activeGoal.goal.goalCode,
      }),
      t("chat.shortTermMemory.messages.goalCleared"),
    );
  }

  async function handleCreateSession() {
    if (!activeWorkspaceCode) {
      toast.warning("Create a workspace before starting a chat session", { title: "No workspace selected" });
      return;
    }

    const nextSessionAgentCode = mainChatAgentOption.agentCode;
    setCreatingSession(true);
    try {
      const session = await createChatSession({
        agentCode: nextSessionAgentCode,
        title: "New chat",
        workspaceCode: activeWorkspaceCode,
      });
      setSessionGroups((current) => insertSession(current, session));
      handleSessionSelect(session);
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
    const returnSourceTab = requestedSourceTab ?? activeSourceTab;
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
        selectSessionAfterRemoval(nextGroups, returnSourceTab);
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
    const returnSourceTab = requestedSourceTab ?? activeSourceTab;

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
        selectSessionAfterRemoval(nextGroups, returnSourceTab);
      }
      forgetSessionModelSelection(deleted.sessionCode);
      // Clear MainAgentChatWindow's persisted session if it matches the deleted one
      forgetMainAgentPersistedSession(activeWorkspaceCode, deleted.sessionCode);
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
        activeSessionCode={activeSession?.sessionCode ?? activeSessionCode}
        activeSourceTab={activeSourceTab}
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
        onSourceTabSelect={handleSourceTabSelect}
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
            ) : null}
          </div>
        </header>

        <TronChatPanel
          autoFocusComposer={shouldAutoFocusChatComposer()}
          composerRef={composerRef}
          createSessionLabel="Create session"
          currentUserDisplayName={currentUserDisplayName}
          debugOptions={transcriptDebugOptions}
          draft={composer}
          emptyDescription={activeWorkspaceCode
            ? "Create a new session or pick one from the left panel to view chat details and start messaging."
            : "Select or create a workspace before loading chat sessions."}
          footerAddon={transcriptFooterAddon}
          inputPlaceholder="Enter to send, Shift+Enter for newline"
          liveAguiAnchorMessageCodes={liveAguiAnchorMessageCodes}
          liveAguiEvents={liveAguiEvents}
          liveAguiPending={liveAguiPending}
          messages={visibleMessages}
          onCreateSession={() => {
            void handleCreateSession();
          }}
          onDraftChange={setComposer}
          onEditMessage={setComposer}
          onSend={() => {
            void handleSend();
          }}
          onStop={handleAbortSend}
          protocolEvents={aguiProtocolEvents}
          sendDisabled={!canAbortCurrentSession && (sending || !activeSession || !selectedModel)}
          sending={canAbortCurrentSession}
          session={activeSession}
          showHeader={false}
          title={activeSession?.title ?? "Chat Management"}
        />
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
