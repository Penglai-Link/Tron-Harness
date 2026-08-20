import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
} from "react";
import { useNavigate, useSearchParams } from "react-router";
import {
  AlarmClock,
  ArrowDown,
  Archive,
  BookmarkPlus,
  Bot,
  BrainCircuit,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  CircleDot,
  Copy,
  Ellipsis,
  Eraser,
  Hammer,
  ListTodo,
  LoaderCircle,
  MessageSquareText,
  Pause,
  Pencil,
  Paperclip,
  Play,
  Plus,
  RefreshCw,
  Send,
  Sparkles,
  Square,
  Terminal,
  UserRound,
  type LucideIcon,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import {
  ConfirmDialog,
  InfoTooltip,
  MarkdownContent,
  useWorkspaceSpaces,
  useConsoleShellConfig,
  useDismissibleLayer,
  useOpenRightSidebar,
  useToast,
  PageView,
  type TargetPageFrameInspectionResult,
} from "@/shared";
import { cn } from "@/shared/lib/utils";
import {
  archiveChatSession,
  createChatSession,
  deleteChatSession,
  getChatActiveRun,
  stopChatRun,
  steerChatMessage,
  streamChatMessage,
  updateChatSession,
  useChatAguiEvents,
  useChatAguiProtocolEvents,
  useChatActiveRunState,
  createChatCompaction,
  cancelChatSessionGoal,
  clearChatSessionGoal,
  pauseChatSessionGoal,
  resumeChatSessionGoal,
  setChatSessionGoal,
  updateChatSessionGoal,
  useChatLatestCompactionState,
  useChatMessageListBySession,
  useChatSessionLatestGoalState,
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
import type { ModelRecord, ReasoningEffort } from "@/domain/model/model";
import {
  formatReasoningEffort,
  reasoningEffortOptions,
} from "@/domain/model/constants/modelPageOptions";
import { isApiError } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  resolveAguiDebugEventTypes,
  shouldShowAguiEventType,
  useAguiDebugPreferences,
  type AguiDebugPreferences,
} from "@/domain/settings/model/aguiDebugPreferences";
import { useAguiWorkReplayPreferences } from "@/domain/settings/model/aguiWorkReplayPreferences";
import { TronChatPanel } from "@tron/chat-react";
import {
  enqueueChatSubmission,
  isSteerQueueFallbackCode,
  mergeChatMessagesChronologically,
  removeQueuedChatSubmission,
  takeNextQueuedChatSubmission,
  type ChatQueuedSubmission,
  type ChatSubmitMode,
} from "@tron/chat-core";
import { deriveReasoningPreview, truncateLabel } from "./chatReasoningPreview";
import {
  isChatReplayRunTerminal,
  resolveLatestOpenChatReplayRun,
} from "../model/chatReplayRun";
import { resolveChatRunUiState } from "../model/chatRunUiState";
import { inferChatAgentCodeFromModel } from "../model/chatAgentCode";
import { parseGoalCommand } from "../model/goalCommand";
import {
  canClearGoalStatus,
  canStartNewGoalFromStatus,
  isEditableGoalStatus,
} from "../model/goalStatusState";
import type {
  ChatAguiEvent,
  ChatAguiStreamEvent,
  ChatMessage,
  ChatSessionCompaction,
  ChatSessionGoalView,
  ChatSessionGoalStatus,
  ChatSessionGoalCommand,
  ChatSession,
  ChatSessionGroup,
  ChatSessionTodo,
  ChatSessionTodoPriority,
  ChatSessionTodoStatus,
  ChatMessageAttachment,
  ChatContextBudgetBreakdown,
  ChatTokenEstimateSnapshot,
  ChatTokenUsage,
  ChatUsageAuditSnapshot,
  SendChatMessageInput,
  SetChatSessionGoalInput,
  UpdateChatSessionGoalInput,
} from "@/domain/chat/model";
import { EditableTabs, type EditableTab } from "@/components/editable-tabs";
import { useInspection } from "../hooks/useInspection";
import {
  downloadChatSessionAttachment,
  uploadChatSessionAttachment,
} from "../api/chatAttachmentApi";
import DraggleLine from "@/shared/ui/DraggleLine";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { throttle } from "lodash-es";

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
const modelDefaultReasoningEffortValue = "__model_default__";

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

function isChatManagementPathname(pathname: string) {
  return pathname === "/chats" || pathname.startsWith("/chats/");
}

function ChatAttachmentChip({
  attachment,
  downloadAttachment,
  onRemove,
}: {
  attachment: ChatMessageAttachment;
  downloadAttachment?: (attachment: ChatMessageAttachment) => Promise<Blob>;
  onRemove: () => void;
}) {
  const label = attachment.fileName || attachment.title || attachment.summary || attachment.id;
  const contentType = attachment.contentType?.toLowerCase();
  const isImage = attachment.kind === "image"
    || Boolean(attachment.imageUrl)
    || contentType?.startsWith("image/");
  const [imageSource, setImageSource] = useState<string | null>(attachment.imageUrl ?? null);

  useEffect(() => {
    if (!isImage || attachment.imageUrl || !downloadAttachment) {
      setImageSource(attachment.imageUrl ?? null);
      return;
    }
    let active = true;
    let objectUrl: string | null = null;
    void downloadAttachment(attachment)
      .then((blob) => {
        const nextObjectUrl = URL.createObjectURL(blob);
        if (!active) {
          URL.revokeObjectURL(nextObjectUrl);
          return;
        }
        objectUrl = nextObjectUrl;
        setImageSource(nextObjectUrl);
      })
      .catch(() => {
        if (active) setImageSource(null);
      });
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [attachment, downloadAttachment, isImage]);

  if (isImage) {
    return (
      <div className="group relative h-16 w-16 overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)]">
        {imageSource ? (
          <img alt={label} className="h-full w-full object-cover" src={imageSource} />
        ) : null}
        <Button
          aria-label={`Remove ${label}`}
          size="icon"
          variant="outline"
          className="absolute right-1 top-1 h-6 w-6 rounded-full bg-[var(--color-on-surface)] p-0 text-[var(--color-surface)]"
          onClick={onRemove}
        >
          <X size={14} />
        </Button>
      </div>
    );
  }

  const fileName = attachment.fileName || attachment.title || label;
  const extension = fileName.includes(".") ? fileName.split(".").pop() : undefined;
  const typeLabel = (extension || contentType?.split("/").pop() || "FILE").toUpperCase();
  return (
    <div className="relative flex h-16 w-[min(280px,100%)] min-w-0 items-center gap-3 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] px-2.5 py-2 pr-10">
      <span className="flex size-11 shrink-0 items-center justify-center rounded-lg bg-[var(--color-surface-container-low)] text-[var(--color-on-surface-variant)]">
        <Paperclip aria-hidden="true" size={20} />
      </span>
      <span className="flex min-w-0 flex-col gap-0.5 text-left">
        <span className="truncate text-sm font-semibold text-[var(--color-on-surface)]" title={label}>{label}</span>
        <span className="truncate text-xs text-[var(--color-on-surface-variant)]">{typeLabel}</span>
      </span>
      <Button
        aria-label={`Remove ${label}`}
        size="icon"
        variant="outline"
        className="absolute right-1 top-1 h-6 w-6 rounded-full bg-[var(--color-on-surface)] p-0 text-[var(--color-surface)]"
        onClick={onRemove}
      >
        <X size={14} />
      </Button>
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
  // 提取不含查询参数的 sessionCode
  const cleanSessionCode = session.sessionCode.split("?")[0];
  return [
    session.sourceType ?? "Manual",
    session.sourceCode ?? "",
    session.taskCode ?? "",
    session.taskCode ? session.taskCode : "Chat",
    cleanSessionCode,
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
  if (value == null) return "-";
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
    messageTokenEstimate: parsed.messageTokenEstimate,
    modelCode: parsed.modelCode ?? "",
    modelName: parsed.modelName ?? "",
    modelProtocol: parsed.modelProtocol ?? "",
    outputTokenReserve: tokenField(parsed.outputTokenReserve),
    overBudget: Boolean(parsed.overBudget),
    compactionTriggerReached: Boolean(parsed.compactionTriggerReached),
    rebased: Boolean(parsed.rebased),
    toolSchemaTokenEstimate: parsed.toolSchemaTokenEstimate,
    usageConfidence: parsed.usageConfidence ?? "estimated",
    usageSource: parsed.usageSource ?? "local_estimate",
  };
}

function normalizeContextBudgetBreakdown(parsed: Partial<ChatContextBudgetBreakdown> | null | undefined): ChatContextBudgetBreakdown | null {
  if (typeof parsed?.estimatedInputTokens !== "number") return null;
  return {
    checkpointBudgetTokens: tokenField(parsed.checkpointBudgetTokens),
    checkpointTokens: tokenField(parsed.checkpointTokens) ?? 0,
    compressedHistoryMessageCount: tokenField(parsed.compressedHistoryMessageCount),
    compressedHistoryTokens: tokenField(parsed.compressedHistoryTokens) ?? 0,
    compactionTriggerRatioBps: tokenField(parsed.compactionTriggerRatioBps),
    compactionTriggerReached: Boolean(parsed.compactionTriggerReached),
    compactionTriggerTokens: tokenField(parsed.compactionTriggerTokens),
    contextWindowLimit: tokenField(parsed.contextWindowLimit),
    currentSessionHistoryBudgetTokens: tokenField(parsed.currentSessionHistoryBudgetTokens),
    effectiveInputBudget: tokenField(parsed.effectiveInputBudget),
    effectiveRetainedHistoryTokens: tokenField(parsed.effectiveRetainedHistoryTokens),
    estimatedInputTokens: parsed.estimatedInputTokens,
    historySourceMessageCount: tokenField(parsed.historySourceMessageCount),
    inputBudgetExceeded: Boolean(parsed.inputBudgetExceeded),
    outputTokenReserve: tokenField(parsed.outputTokenReserve),
    promptPrefixMessageCount: tokenField(parsed.promptPrefixMessageCount),
    promptPrefixBudgetTokens: tokenField(parsed.promptPrefixBudgetTokens),
    promptPrefixTokens: tokenField(parsed.promptPrefixTokens) ?? 0,
    rawTailMessageCount: tokenField(parsed.rawTailMessageCount),
    rawTailStartSeq: tokenField(parsed.rawTailStartSeq),
    rawTailTranscriptTokens: tokenField(parsed.rawTailTranscriptTokens) ?? 0,
    recentTranscriptTokens: tokenField(parsed.recentTranscriptTokens) ?? 0,
    retainedHistoryRatioBps: tokenField(parsed.retainedHistoryRatioBps),
    sessionFamilyContextBudgetTokens: tokenField(parsed.sessionFamilyContextBudgetTokens),
    sessionFamilyContextTokens: tokenField(parsed.sessionFamilyContextTokens) ?? 0,
    toolSchemaTokens: tokenField(parsed.toolSchemaTokens) ?? 0,
    workingStateBudgetTokens: tokenField(parsed.workingStateBudgetTokens),
    workingStateTokens: tokenField(parsed.workingStateTokens) ?? 0,
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

function parsedContextBudgetBreakdown(value: string | null | undefined): ChatContextBudgetBreakdown | null {
  if (!value) return null;
  try {
    return normalizeContextBudgetBreakdown(JSON.parse(value) as Partial<ChatContextBudgetBreakdown>);
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
  return parsedUsageEstimate(latestEventValue(events, "usage_estimate_json"))
    ?? parsedUsageEstimate(latestEventValue(events, "usageEstimate"));
}

function latestUsageAuditFromEvents(events: ChatAguiEvent[]) {
  return parsedUsageAudit(latestEventValue(events, "usage_audit_json"));
}

function latestContextBudgetBreakdownFromEvents(events: ChatAguiEvent[]) {
  return parsedContextBudgetBreakdown(latestEventValue(events, "budgetBreakdown"));
}

function chatTokenUsage(messages: ChatMessage[]) {
  const assistantMessages = messages.filter((message) => message.role === "ASSISTANT");
  let hasUsage = false;
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

function insertNewlineAtCursor(textarea: HTMLTextAreaElement) {
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const nextValue = `${textarea.value.slice(0, start)}\n${textarea.value.slice(end)}`;
  textarea.value = nextValue;
  textarea.selectionStart = start + 1;
  textarea.selectionEnd = start + 1;
  return nextValue;
}

function formatCompactTokenCount(value: number | null | undefined) {
  if (value == null) return "-";
  const absolute = Math.abs(value);
  if (absolute >= 1_000_000) {
    return `${new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 }).format(value / 1_000_000)}m`;
  }
  if (absolute >= 1_000) {
    return `${new Intl.NumberFormat("en-US", { maximumFractionDigits: absolute < 10_000 ? 1 : 0 }).format(value / 1_000)}k`;
  }
  return new Intl.NumberFormat("en-US").format(value);
}

function formatTokenLimit(usedTokens: number | null | undefined, limitTokens: number | null | undefined) {
  const used = formatCompactTokenCount(usedTokens);
  const limit = limitTokens == null ? "-" : formatCompactTokenCount(limitTokens);
  return `${used} / ${limit}`;
}

function contextPercent(usedTokens: number | null | undefined, limitTokens: number | null | undefined) {
  if (usedTokens == null || limitTokens == null || limitTokens <= 0) return 0;
  return Math.min(100, Math.max(0, (usedTokens / limitTokens) * 100));
}

function formatPercent(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "-";
  return `${new Intl.NumberFormat("en-US", { maximumFractionDigits: value < 10 ? 1 : 0 }).format(value)}%`;
}

function formatSignedPercentFromRatio(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "-";
  const percent = (value - 1) * 100;
  const sign = percent > 0 ? "+" : "";
  return `${sign}${new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 }).format(percent)}%`;
}

function safeSubtract(minuend: number | null | undefined, subtrahend: number | null | undefined) {
  if (minuend == null || subtrahend == null) return null;
  return Math.max(0, minuend - subtrahend);
}

function contextStatusTone(estimate: ChatTokenEstimateSnapshot | null, breakdown: ChatContextBudgetBreakdown | null): BadgeToneName {
  if (estimate?.overBudget || breakdown?.inputBudgetExceeded) return "error";
  if (estimate?.compactionTriggerReached || breakdown?.compactionTriggerReached) return "warning";
  return "success";
}

function contextStatusLabel(estimate: ChatTokenEstimateSnapshot | null, breakdown: ChatContextBudgetBreakdown | null, translate: (key: string) => string) {
  if (estimate?.overBudget || breakdown?.inputBudgetExceeded) return translate("chat.shortTermMemory.contextPanel.overBudget");
  if (estimate?.compactionTriggerReached || breakdown?.compactionTriggerReached) return translate("chat.shortTermMemory.contextPanel.nearCompaction");
  return translate("chat.shortTermMemory.contextPanel.healthy");
}

function contextWindowLimitFromSources(
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

type TranscriptAguiEvent = ChatAguiEvent | ChatAguiStreamEvent;

export interface ChatStreamOwner {
  controller: AbortController;
  generation: number;
  sessionCode: string;
}

export class ChatStreamOwnershipRegistry {
  private activeOwner: ChatStreamOwner | null = null;
  private nextGeneration = 0;

  get current() {
    return this.activeOwner;
  }

  acquire(sessionCode: string, controller: AbortController) {
    this.nextGeneration += 1;
    const owner: ChatStreamOwner = {
      controller,
      generation: this.nextGeneration,
      sessionCode,
    };
    this.activeOwner = owner;
    return owner;
  }

  owns(owner: ChatStreamOwner) {
    return this.activeOwner === owner;
  }

  detach(sessionCode?: string) {
    const owner = this.activeOwner;
    if (!owner || (sessionCode && owner.sessionCode !== sessionCode)) {
      return null;
    }
    this.activeOwner = null;
    return owner;
  }

  release(owner: ChatStreamOwner) {
    if (!this.owns(owner)) return false;
    this.activeOwner = null;
    return true;
  }
}

export interface ChatStreamSettlement {
  canCommit: boolean;
  restoreSourceSubmission: boolean;
}

export function resolveChatStreamSettlement(args: {
  activeSessionCode: string;
  detached: boolean;
  owner: ChatStreamOwner;
  registry: ChatStreamOwnershipRegistry;
  stopped: boolean;
}): ChatStreamSettlement {
  if (args.stopped) {
    return { canCommit: false, restoreSourceSubmission: false };
  }
  const canCommit = !args.detached
    && args.registry.owns(args.owner)
    && args.activeSessionCode === args.owner.sessionCode;
  return {
    canCommit,
    restoreSourceSubmission: false,
  };
}

const chatScrollBottomThreshold = 96;
const chatScrollJumpButtonThreshold = 160;
const liveAguiStartToleranceMs = 5_000;
const activeChatReplayPollIntervalMs = 1_000;
const activeChatReplayPendingTurnMaxAgeMs = 30 * 60_000;

export function shouldPollChatRunRecovery({
  currentRunActive,
  hasQueuedSubmission,
  sessionCode,
  shouldPollData,
}: {
  currentRunActive: boolean;
  hasQueuedSubmission: boolean;
  sessionCode: string;
  shouldPollData: boolean;
}) {
  return Boolean(
    sessionCode
    && (currentRunActive || hasQueuedSubmission || shouldPollData),
  );
}

export function claimChatRunTerminalDrain(
  drainedRunKeys: Set<string>,
  sessionCode: string,
  runCode: string,
) {
  if (!sessionCode || !runCode) return false;
  const drainKey = `${sessionCode}:${runCode}`;
  if (drainedRunKeys.has(drainKey)) return false;
  drainedRunKeys.add(drainKey);
  return true;
}

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

export function latestTerminalChatRunCode(
  events: TranscriptAguiEvent[],
  sessionCode: string,
) {
  const terminalEvents = events
    .map((event, index) => {
      const occurredAt = "occurredAt" in event && typeof event.occurredAt === "string"
        ? Date.parse(event.occurredAt)
        : Number.NaN;
      return {
        event,
        index,
        timestamp: "timestamp" in event
          && typeof event.timestamp === "number"
          && Number.isFinite(event.timestamp)
          ? event.timestamp
          : (Number.isNaN(occurredAt) ? null : occurredAt),
      };
    })
    .filter(({ event }) => {
      const threadId = aguiEventThreadId(event);
      if (threadId && threadId !== sessionCode) return false;
      const runCode = aguiEventRunCode(event);
      if (!runCode || runCode === "stream") return false;
      const type = aguiEventType(event).toUpperCase();
      return type === "RUN_FINISHED" || type === "RUN_ERROR";
    });
  if (!terminalEvents.length) return "";
  const ordered = terminalEvents.every(({ timestamp }) => timestamp !== null)
    ? terminalEvents.sort((left, right) => (
      (left.timestamp as number) - (right.timestamp as number)
      || left.index - right.index
    ))
    : terminalEvents;
  return aguiEventRunCode(ordered[ordered.length - 1].event);
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

function isChatRunAlreadyActiveError(error: unknown) {
  return isApiError(error) && error.code === "CHAT_RUN_ALREADY_ACTIVE";
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
    "inline-flex flex-none size-8 items-center justify-center rounded-md border border-transparent text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50",
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

type BadgeToneName = "secondary" | "info" | "success" | "warning" | "error";

function goalStatusTone(status: ChatSessionGoalStatus): BadgeToneName {
  if (status === "BLOCKED") return "warning";
  if (status === "COMPLETED") return "success";
  if (status === "CANCELLED" || status === "CLEARED") return "secondary";
  return "secondary";
}

function todoStatusTone(status: ChatSessionTodoStatus): BadgeToneName {
  if (status === "completed") return "success";
  if (status === "cancelled") return "secondary";
  if (status === "in_progress") return "info";
  return "secondary";
}

function todoPriorityTone(priority: ChatSessionTodoPriority): BadgeToneName {
  if (priority === "high") return "warning";
  return "secondary";
}

function todoStatusDotClass(status: ChatSessionTodoStatus) {
  if (status === "in_progress") return "bg-[var(--color-focus-ring)]";
  if (status === "completed") return "bg-[var(--color-success)]";
  if (status === "cancelled") return "bg-[var(--color-outline-variant)]";
  return "bg-[var(--color-outline)]";
}

function isFoldedTodo(todo: ChatSessionTodo) {
  return todo.status === "completed" || todo.status === "cancelled";
}

function parseGoalMaxRunsDraft(value: string, fallback: number) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? Math.max(1, parsed) : fallback;
}

function WorkspaceTabState({
  goal,
  running,
}: {
  goal?: ChatSessionGoalView | null;
  running: boolean;
}) {
  const { t } = useTranslation("pages");
  const status = goal?.goal.status;
  if (!status) return null;
  const label = t(`chat.shortTermMemory.goalStatuses.${status}`);

  if (status === "ACTIVE" && running) {
    return (
      <span className="inline-flex size-4 items-center justify-center text-[var(--color-focus-ring)]" title={label} aria-label={label}>
        <LoaderCircle className="animate-spin" size={14} />
      </span>
    );
  }

  if (status === "ACTIVE") {
    return (
      <span className="inline-flex size-4 items-center justify-center text-[var(--color-focus-ring)]" title={label} aria-label={label}>
        <CircleDot size={14} />
      </span>
    );
  }

  if (status === "BLOCKED") {
    return (
      <span className="inline-flex size-4 items-center justify-center text-[var(--color-warning)]" title={label} aria-label={label}>
        <CircleAlert size={14} />
      </span>
    );
  }

  if (status === "PAUSED") {
    return (
      <span className="inline-flex size-4 items-center justify-center text-[var(--color-on-surface-variant)]" title={label} aria-label={label}>
        <Pause size={14} />
      </span>
    );
  }

  if (status === "COMPLETED") {
    return (
      <span className="inline-flex size-4 items-center justify-center text-[var(--color-success)]" title={label} aria-label={label}>
        <CheckCircle2 size={14} />
      </span>
    );
  }

  return null;
}

function GoalDetailBlock({
  children,
  title,
  tone = "neutral",
}: {
  children: React.ReactNode;
  title: string;
  tone?: "neutral" | "warning";
}) {
  return (
    <div
      className={cn(
        "space-y-1 border-l-2 border-[var(--color-outline-variant)] pl-3",
        tone === "warning" && "border-[var(--color-warning)]",
      )}
    >
      <div className="text-[11px] font-medium text-[var(--color-on-surface-variant)]">{title}</div>
      <div className="max-h-24 overflow-y-auto whitespace-pre-wrap break-words pr-2 text-xs leading-5 text-[var(--color-on-surface)]">
        {children}
      </div>
    </div>
  );
}

function goalVerdictLabel(value: string | null | undefined, translate: (key: string) => string) {
  const normalized = value?.trim().toUpperCase();
  if (!normalized) return translate("chat.shortTermMemory.goalVerdicts.UNKNOWN");
  if (normalized === "CONTINUE") return translate("chat.shortTermMemory.goalVerdicts.CONTINUE");
  if (normalized === "COMPLETE" || normalized === "COMPLETED") return translate("chat.shortTermMemory.goalVerdicts.COMPLETED");
  if (normalized === "BLOCKED") return translate("chat.shortTermMemory.goalVerdicts.BLOCKED");
  if (normalized === "FAILED" || normalized === "ERROR") return translate("chat.shortTermMemory.goalVerdicts.FAILED");
  return value?.trim() ?? normalized;
}

type ContextBlockColor = "seed" | "tool" | "checkpoint" | "state" | "family" | "history";

interface ContextMemoryBlockItem {
  code: string;
  color: ContextBlockColor;
  limitTokens: number | null;
  meta: string;
  name: string;
  stat: string;
  tokens: number;
}

const mutedContextBlockColor = {
  dot: "border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)]",
  fill: "bg-[var(--color-on-surface-variant)]",
};

const contextBlockColorClasses: Record<ContextBlockColor, { dot: string; fill: string }> = {
  checkpoint: mutedContextBlockColor,
  family: mutedContextBlockColor,
  history: mutedContextBlockColor,
  seed: mutedContextBlockColor,
  state: mutedContextBlockColor,
  tool: mutedContextBlockColor,
};

function buildContextMemoryBlocks({
  breakdown,
  goal,
  latestCompaction,
  requestEstimate,
  todos,
  translate,
}: {
  breakdown: ChatContextBudgetBreakdown | null;
  goal?: ChatSessionGoalView | null;
  latestCompaction?: ChatSessionCompaction | null;
  requestEstimate: ChatTokenEstimateSnapshot | null;
  todos?: ChatSessionTodo[];
  translate: (key: string, options?: Record<string, unknown>) => string;
}): ContextMemoryBlockItem[] {
  const activeTodoCount = (todos ?? []).filter((todo) => !isFoldedTodo(todo)).length;
  const activeGoalCount = goal && isEditableGoalStatus(goal.goal.status) ? 1 : 0;
  const historyMessageCount = breakdown?.rawTailMessageCount
    ?? breakdown?.historySourceMessageCount
    ?? 0;
  const checkpointTokens = breakdown?.checkpointTokens ?? 0;
  const checkpointCode = contextValue(requestEstimate?.checkpointCode);
  const hasCheckpointInRequest = Boolean(latestCompaction || checkpointCode || checkpointTokens > 0);
  const checkpointMeta = latestCompaction
    ? translate("chat.shortTermMemory.contextPanel.checkpointMeta", {
      end: latestCompaction.sourceEndSeq,
      start: latestCompaction.sourceStartSeq,
    })
    : checkpointCode
      ? translate("chat.shortTermMemory.contextPanel.checkpointCodeMeta", { code: checkpointCode })
      : checkpointTokens > 0
        ? translate("chat.shortTermMemory.contextPanel.checkpointLoadedMeta")
        : translate("chat.shortTermMemory.contextPanel.noCheckpointMeta");
  const checkpointStat = latestCompaction || checkpointCode
    ? translate("chat.shortTermMemory.contextPanel.checkpointStat")
    : checkpointTokens > 0
      ? translate("chat.shortTermMemory.contextPanel.loaded")
      : translate("chat.shortTermMemory.contextPanel.none");
  const sharedInputBudget = breakdown?.effectiveInputBudget ?? requestEstimate?.effectiveInputBudget ?? null;
  const toolSchemaTokens = breakdown?.toolSchemaTokens ?? requestEstimate?.toolSchemaTokenEstimate ?? 0;
  const toolSchemaLimit = sharedInputBudget ?? breakdown?.contextWindowLimit ?? requestEstimate?.contextWindowLimit ?? null;
  const compressedCount = breakdown?.compressedHistoryMessageCount ?? 0;
  const rawTailCount = breakdown?.rawTailMessageCount ?? 0;
  const historyMeta = compressedCount > 0
    ? translate("chat.shortTermMemory.contextPanel.compressedHistoryMeta", {
      compressed: compressedCount,
      tail: rawTailCount,
    })
    : translate("chat.shortTermMemory.contextPanel.historyMeta");

  return [
    {
      code: "01",
      color: "seed",
      limitTokens: breakdown?.promptPrefixBudgetTokens ?? null,
      meta: translate("chat.shortTermMemory.contextPanel.seedMeta"),
      name: translate("chat.shortTermMemory.contextPanel.systemPromptName"),
      stat: translate("chat.shortTermMemory.contextPanel.messageCount", {
        count: breakdown?.promptPrefixMessageCount ?? 0,
      }),
      tokens: breakdown?.promptPrefixTokens ?? 0,
    },
    {
      code: "02",
      color: "tool",
      limitTokens: toolSchemaLimit,
      meta: translate("chat.shortTermMemory.contextPanel.toolMeta"),
      name: "Tool Schema",
      stat: "",
      tokens: toolSchemaTokens,
    },
    {
      code: "03",
      color: "checkpoint",
      limitTokens: breakdown?.checkpointBudgetTokens ?? null,
      meta: checkpointMeta,
      name: "Current Session Checkpoint",
      stat: hasCheckpointInRequest ? checkpointStat : translate("chat.shortTermMemory.contextPanel.none"),
      tokens: checkpointTokens,
    },
    {
      code: "04",
      color: "state",
      limitTokens: breakdown?.workingStateBudgetTokens ?? null,
      meta: translate("chat.shortTermMemory.contextPanel.workingStateMeta"),
      name: "Current Session Working State",
      stat: translate("chat.shortTermMemory.contextPanel.workingStateStat", {
        goals: activeGoalCount,
        todos: activeTodoCount,
      }),
      tokens: breakdown?.workingStateTokens ?? 0,
    },
    {
      code: "05",
      color: "family",
      limitTokens: breakdown?.sessionFamilyContextBudgetTokens ?? null,
      meta: translate("chat.shortTermMemory.contextPanel.familyMeta"),
      name: "Session Family Context",
      stat: breakdown?.sessionFamilyContextTokens
        ? translate("chat.shortTermMemory.contextPanel.loaded")
        : translate("chat.shortTermMemory.contextPanel.none"),
      tokens: breakdown?.sessionFamilyContextTokens ?? 0,
    },
    {
      code: "06",
      color: "history",
      limitTokens: breakdown?.currentSessionHistoryBudgetTokens ?? null,
      meta: historyMeta,
      name: "Current Session History",
      stat: translate("chat.shortTermMemory.contextPanel.messageCount", { count: historyMessageCount }),
      tokens: breakdown?.recentTranscriptTokens ?? requestEstimate?.messageTokenEstimate ?? 0,
    },
  ];
}

function ContextOverviewPanel({
  breakdown,
  fallbackWindowLimit,
  modelLabel,
  requestEstimate,
}: {
  breakdown: ChatContextBudgetBreakdown | null;
  fallbackWindowLimit?: number | null;
  modelLabel: string;
  requestEstimate: ChatTokenEstimateSnapshot | null;
}) {
  const { t } = useTranslation("pages");
  const usedTokens = breakdown?.estimatedInputTokens ?? requestEstimate?.estimatedInputTokens ?? (fallbackWindowLimit ? 0 : null);
  const inputBudget = breakdown?.effectiveInputBudget ?? requestEstimate?.effectiveInputBudget ?? fallbackWindowLimit ?? null;
  const windowLimit = breakdown?.contextWindowLimit ?? requestEstimate?.contextWindowLimit ?? fallbackWindowLimit ?? null;
  const percent = contextPercent(usedTokens, inputBudget ?? windowLimit);
  const statusLabel = contextStatusLabel(requestEstimate, breakdown, (key) => t(key));
  const statusTone = contextStatusTone(requestEstimate, breakdown);

  return (
    <section className="space-y-3 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3">
      <div className="flex min-w-0 items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="text-xs font-semibold uppercase text-[var(--color-on-surface-variant)]">
            {t("chat.shortTermMemory.contextPanel.overview")}
          </h3>
          <p className="truncate text-[11px] text-[var(--color-on-surface-variant)]">
            {requestEstimate?.modelName || requestEstimate?.modelCode || modelLabel}
            {windowLimit ? ` · ${formatCompactTokenCount(windowLimit)} ${t("chat.shortTermMemory.contextPanel.windowSuffix")}` : ""}
          </p>
        </div>
        <Badge className="h-5 rounded px-1.5 text-[11px]" tone={statusTone}>
          {statusLabel}
        </Badge>
      </div>
      <div className="flex items-end justify-between gap-3">
        <div className="min-w-0">
          <div className="text-2xl font-semibold text-[var(--color-on-surface)]">
            {formatCompactTokenCount(usedTokens)}
          </div>
          <div className="text-[11px] text-[var(--color-on-surface-variant)]">
            {t("chat.shortTermMemory.contextPanel.inputBudget", {
              budget: formatCompactTokenCount(inputBudget),
            })}
          </div>
        </div>
        <div className="pb-0.5 text-sm font-medium text-[var(--color-on-surface-variant)]">
          {formatPercent(percent)}
        </div>
      </div>
      <div className="h-2.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
        <div
          className={cn(
            "h-full rounded-full transition-[width]",
            statusTone === "error" ? "bg-[var(--color-error)]" : statusTone === "warning" ? "bg-[var(--color-warning)]" : "bg-[var(--color-success)]",
          )}
          style={{ width: `${percent}%` }}
        />
      </div>
    </section>
  );
}

function ContextStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md bg-[var(--color-surface-container)] px-2 py-1.5">
      <div className="truncate text-sm font-semibold text-[var(--color-on-surface)]">{value}</div>
      <div className="truncate text-[10px] text-[var(--color-on-surface-variant)]">{label}</div>
    </div>
  );
}

function ContextStructurePanel({
  blocks,
  compactionBusy,
  onCreateCompaction,
  session,
}: {
  blocks: ContextMemoryBlockItem[];
  compactionBusy?: boolean;
  onCreateCompaction?: () => void;
  session?: ChatSession;
}) {
  const { t } = useTranslation("pages");
  return (
    <section className="space-y-2">
      <div className="flex min-w-0 items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-1">
          <h3 className="truncate text-xs font-semibold uppercase text-[var(--color-on-surface-variant)]">
            {t("chat.shortTermMemory.contextPanel.structure")}
          </h3>
          <InfoTooltip ariaLabel={t("chat.shortTermMemory.contextPanel.budgetFormulaAria")} align="start">
            <div className="max-w-[18rem] space-y-1 whitespace-normal leading-5">
              <p className="font-medium">{t("chat.shortTermMemory.contextPanel.budgetFormulaTitle")}</p>
              <p>{t("chat.shortTermMemory.contextPanel.effectiveInputFormula")}</p>
              <p className="whitespace-pre-line">{t("chat.shortTermMemory.contextPanel.historyFormula")}</p>
              <p className="text-[var(--color-on-surface-variant)]">{t("chat.shortTermMemory.contextPanel.budgetFormulaNote")}</p>
            </div>
          </InfoTooltip>
        </div>
        <button
          aria-label={t("chat.shortTermMemory.createCheckpoint")}
          className={iconButtonClass()}
          disabled={!session || compactionBusy}
          title={t("chat.shortTermMemory.createCheckpoint")}
          type="button"
          onClick={onCreateCompaction}
        >
          {compactionBusy ? <LoaderCircle className="animate-spin" size={15} /> : <BookmarkPlus size={15} />}
        </button>
      </div>
      <div className="space-y-2">
        {blocks.map((block) => (
          <ContextMemoryBlockRow block={block} key={block.code} />
        ))}
      </div>
    </section>
  );
}

function ContextMemoryBlockRow({ block }: { block: ContextMemoryBlockItem }) {
  const colors = contextBlockColorClasses[block.color];
  const percent = contextPercent(block.tokens, block.limitTokens);
  const fallbackPercent = block.limitTokens == null && block.tokens > 0 ? 40 : percent;
  return (
    <div className="rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-2.5">
      <div className="min-w-0">
        <div className="flex min-w-0 items-baseline justify-between gap-2">
          <div className="truncate text-xs font-semibold text-[var(--color-on-surface)]">{block.name}</div>
          <div className="shrink-0 font-mono text-[11px] text-[var(--color-on-surface)]">
            {formatTokenLimit(block.tokens, block.limitTokens)}
          </div>
        </div>
        <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
          <div
            className={cn("h-full rounded-full", colors.fill)}
            style={{ width: `${fallbackPercent}%` }}
          />
        </div>
        <div className="mt-1.5 flex min-w-0 items-center justify-between gap-2 text-[10px] text-[var(--color-on-surface-variant)]">
          <span className="min-w-0 truncate">{block.meta}</span>
          {block.stat ? <span className="shrink-0">{block.stat}</span> : null}
        </div>
      </div>
    </div>
  );
}

function ProviderUsagePanel({
  fallbackSessionUsage,
  usageAudit,
}: {
  fallbackSessionUsage: { input: number | null; output: number | null; total: number | null };
  usageAudit: ChatUsageAuditSnapshot | null;
}) {
  const { t } = useTranslation("pages");
  const providerUsage = usageAudit?.providerUsage ?? null;
  const requestEstimate = usageAudit?.requestEstimate ?? null;
  const actualInput = providerUsage?.inputTokens ?? fallbackSessionUsage.input;
  const cachedInput = providerUsage?.cachedInputTokens ?? null;
  const reasoningTokens = providerUsage?.reasoningTokens ?? null;
  const actualTotal = providerUsage?.totalTokens ?? fallbackSessionUsage.total;
  const inputBudget = requestEstimate?.effectiveInputBudget ?? requestEstimate?.contextWindowLimit ?? null;
  const inputPercent = contextPercent(actualInput, inputBudget);
  const cachedPercent = contextPercent(cachedInput, actualInput);
  const reasoningPercent = contextPercent(reasoningTokens, actualTotal);

  return (
    <section className="space-y-3 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3">
      <div>
        <h3 className="text-xs font-semibold uppercase text-[var(--color-on-surface-variant)]">
          {t("chat.shortTermMemory.contextSections.providerUsage")}
        </h3>
        <p className="text-[11px] text-[var(--color-on-surface-variant)]">
          {t("chat.shortTermMemory.contextPanel.providerSubtitle")}
        </p>
      </div>
      <div className="grid grid-cols-3 gap-1.5">
        <ContextStat label={t("chat.shortTermMemory.contextPanel.actualInput")} value={formatCompactTokenCount(actualInput)} />
        <ContextStat label={t("chat.shortTermMemory.contextPanel.cached")} value={formatCompactTokenCount(cachedInput)} />
        <ContextStat label={t("chat.shortTermMemory.contextPanel.reasoning")} value={formatCompactTokenCount(reasoningTokens)} />
      </div>
      <div className="space-y-2">
        <ProviderUsageLine
          label={t("chat.shortTermMemory.contextPanel.input")}
          percent={inputPercent}
          value={usageAudit?.inputTokenRatio ? formatSignedPercentFromRatio(usageAudit.inputTokenRatio) : formatCompactTokenCount(actualInput)}
        />
        <ProviderUsageLine
          label={t("chat.shortTermMemory.contextPanel.cached")}
          percent={cachedPercent}
          value={formatPercent(cachedInput == null ? null : cachedPercent)}
        />
        <ProviderUsageLine
          label={t("chat.shortTermMemory.contextPanel.reasoning")}
          percent={reasoningPercent}
          value={formatCompactTokenCount(reasoningTokens)}
        />
      </div>
      <div className="border-t border-[var(--color-outline-variant)] pt-2 text-xs leading-5 text-[var(--color-on-surface-variant)]">
        {requestEstimate ? (
          <span>
            {t("chat.shortTermMemory.contextPanel.providerDelta", {
              actual: formatCompactTokenCount(providerUsage?.inputTokens),
              delta: formatCompactTokenCount(usageAudit?.inputTokenDelta),
              estimate: formatCompactTokenCount(requestEstimate.estimatedInputTokens),
            })}
            {usageAudit?.calibrationFactor ? (
              <span> {t("chat.shortTermMemory.contextPanel.calibration", {
                factor: new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 }).format(usageAudit.calibrationFactor),
              })}</span>
            ) : null}
          </span>
        ) : (
          t("chat.shortTermMemory.contextPanel.providerUnavailable")
        )}
      </div>
    </section>
  );
}

function ProviderUsageLine({
  label,
  percent,
  value,
}: {
  label: string;
  percent: number;
  value: string;
}) {
  return (
    <div className="grid grid-cols-[4.25rem_minmax(0,1fr)_3.75rem] items-center gap-2 text-[11px] text-[var(--color-on-surface-variant)]">
      <span className="truncate">{label}</span>
      <span className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
        <span
          className="block h-full rounded-full bg-[var(--color-on-surface-variant)]"
          style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
        />
      </span>
      <span className="truncate text-right font-mono text-[var(--color-on-surface)]">{value}</span>
    </div>
  );
}

function RightWorkspaceSidebar({
  goal,
  goalBusy,
  goalStatus,
  onCancelGoal,
  onClearGoal,
  onCreateGoal,
  onPauseGoal,
  onResumeGoal,
  onUpdateGoal,
  sessionCode,
  todos,
  todosStatus,
}: {
  goal?: ChatSessionGoalView | null;
  goalBusy?: boolean;
  goalStatus?: QueryStatus;
  onCancelGoal?: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onClearGoal?: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onCreateGoal?: (input: SetChatSessionGoalInput) => Promise<boolean> | boolean;
  onPauseGoal?: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onResumeGoal?: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onUpdateGoal?: (input: UpdateChatSessionGoalInput) => Promise<boolean> | boolean;
  sessionCode?: string;
  todos?: ChatSessionTodo[];
  todosStatus?: QueryStatus;
}) {
  const { t } = useTranslation("pages");
  const [creatingGoal, setCreatingGoal] = useState(false);
  const [editingGoal, setEditingGoal] = useState(false);
  const [goalDraft, setGoalDraft] = useState("");
  const [goalMaxRunsDraft, setGoalMaxRunsDraft] = useState("");
  const goalDraftSignature = useMemo(() => JSON.stringify({
    goalCode: goal?.goal.goalCode ?? "",
    maxRuns: goal?.goal.maxRuns ?? 0,
    objective: goal?.goal.objectiveMarkdown ?? "",
  }), [goal?.goal.goalCode, goal?.goal.maxRuns, goal?.goal.objectiveMarkdown]);
  const activeTodos = (todos ?? []).filter((todo) => !isFoldedTodo(todo));
  const foldedTodoCount = (todos ?? []).length - activeTodos.length;
  const hasGoalLoadState = Boolean(goalStatus?.loading || goalStatus?.error);
  const hasTodoLoadState = Boolean(todosStatus?.loading || todosStatus?.error);
  const goalStatusLabel = goal ? t(`chat.shortTermMemory.goalStatuses.${goal.goal.status}`) : "";
  const canCreateGoal = Boolean(sessionCode && onCreateGoal && (!goal || canStartNewGoalFromStatus(goal.goal.status)));
  const canEditGoal = Boolean(goal && onUpdateGoal && isEditableGoalStatus(goal.goal.status));
  const showCreateGoalForm = canCreateGoal && (creatingGoal || !goal);

  useEffect(() => {
    if (creatingGoal || editingGoal) return;
    setGoalDraft(goal?.goal.objectiveMarkdown ?? "");
    setGoalMaxRunsDraft(goal ? String(goal.goal.maxRuns) : "20");
  }, [creatingGoal, editingGoal, goalDraftSignature]);

  function resetGoalDraft() {
    setGoalDraft(goal?.goal.objectiveMarkdown ?? "");
    setGoalMaxRunsDraft(goal ? String(goal.goal.maxRuns) : "20");
  }

  function resetCreateGoalDraft() {
    setGoalDraft("");
    setGoalMaxRunsDraft("20");
  }

  function startGoalCreate() {
    resetCreateGoalDraft();
    setEditingGoal(false);
    setCreatingGoal(true);
  }

  function startGoalEdit() {
    resetGoalDraft();
    setCreatingGoal(false);
    setEditingGoal(true);
  }

  function cancelGoalEdit() {
    resetGoalDraft();
    setEditingGoal(false);
  }

  function cancelGoalCreate() {
    resetGoalDraft();
    setCreatingGoal(false);
  }

  async function handleCreateGoal() {
    if (!sessionCode || !onCreateGoal) return;
    const objective = goalDraft.trim();
    if (!objective) return;
    const saved = await onCreateGoal({
      sessionCode,
      objective,
      maxRuns: parseGoalMaxRunsDraft(goalMaxRunsDraft, 20),
    });
    if (saved !== false) {
      resetCreateGoalDraft();
      setCreatingGoal(false);
    }
  }

  async function handleSaveGoal() {
    if (!goal || !onUpdateGoal) return;
    const objective = goalDraft.trim();
    if (!objective) return;
    const saved = await onUpdateGoal({
      sessionCode: goal.goal.sessionCode,
      goalCode: goal.goal.goalCode,
      objective,
      maxRuns: parseGoalMaxRunsDraft(goalMaxRunsDraft, goal.goal.maxRuns),
    });
    if (saved !== false) setEditingGoal(false);
  }

  function goalCommandInput(): ChatSessionGoalCommand | null {
    if (!goal) return null;
    return {
      sessionCode: goal.goal.sessionCode,
      goalCode: goal.goal.goalCode,
    };
  }

  function runGoalCommand(action?: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean) {
    const input = goalCommandInput();
    if (!input || !action) return;
    void action(input);
  }

  function renderGoalForm(mode: "create" | "edit") {
    const savingLabel = mode === "create"
      ? t("chat.shortTermMemory.setGoal")
      : t("chat.shortTermMemory.saveGoal");
    const onSave = mode === "create" ? handleCreateGoal : handleSaveGoal;
    const onCancel = mode === "create" ? cancelGoalCreate : cancelGoalEdit;
    const showCancel = mode === "edit" || Boolean(goal);

    return (
      <div className="space-y-3">
        <textarea
          className="min-h-28 w-full resize-y rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 py-1.5 text-sm text-[var(--color-on-surface)] outline-none transition focus:border-[var(--color-primary)]"
          disabled={goalBusy}
          placeholder={t("chat.shortTermMemory.goalPlaceholder")}
          value={goalDraft}
          onChange={(event) => setGoalDraft(event.target.value)}
        />
        <label className="block space-y-1">
          <span className="text-[11px] font-medium text-[var(--color-on-surface-variant)]">
            {t("chat.shortTermMemory.goalMaxRuns")}
          </span>
          <input
            className="block h-8 w-24 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 text-sm text-[var(--color-on-surface)] outline-none transition focus:border-[var(--color-primary)]"
            disabled={goalBusy}
            min={1}
            type="number"
            value={goalMaxRunsDraft}
            onChange={(event) => setGoalMaxRunsDraft(event.target.value)}
          />
        </label>
        <div className="flex flex-wrap items-center gap-1.5">
          <button
            className={smallActionButtonClass()}
            disabled={goalBusy || !goalDraft.trim()}
            type="button"
            onClick={() => void onSave()}
          >
            {goalBusy ? <LoaderCircle className="animate-spin" size={13} /> : <CheckCircle2 size={13} />}
            <span className="ml-1.5">{goalBusy ? t("chat.shortTermMemory.savingGoal") : savingLabel}</span>
          </button>
          {showCancel ? (
            <button
              className={smallActionButtonClass()}
              disabled={goalBusy}
              type="button"
              onClick={onCancel}
            >
              <X size={13} />
              <span className="ml-1.5">{t("chat.shortTermMemory.cancelEditGoal")}</span>
            </button>
          ) : null}
        </div>
      </div>
    );
  }

  function renderGoalActionButtons() {
    if (!goal || editingGoal || showCreateGoalForm) return null;
    const status = goal.goal.status;
    return (
      <div className="flex flex-wrap items-center gap-1.5">
        {status === "ACTIVE" && onPauseGoal ? (
          <button
            className={smallActionButtonClass()}
            disabled={goalBusy}
            type="button"
            onClick={() => runGoalCommand(onPauseGoal)}
          >
            <Pause size={13} />
            <span className="ml-1.5">{t("chat.shortTermMemory.pauseGoal")}</span>
          </button>
        ) : null}
        {(status === "PAUSED" || status === "BLOCKED") && onResumeGoal ? (
          <button
            className={smallActionButtonClass()}
            disabled={goalBusy}
            type="button"
            onClick={() => runGoalCommand(onResumeGoal)}
          >
            <Play size={13} />
            <span className="ml-1.5">{t("chat.shortTermMemory.resumeGoal")}</span>
          </button>
        ) : null}
        {isEditableGoalStatus(status) && onCancelGoal ? (
          <button
            className={smallActionButtonClass()}
            disabled={goalBusy}
            type="button"
            onClick={() => runGoalCommand(onCancelGoal)}
          >
            <X size={13} />
            <span className="ml-1.5">{t("chat.shortTermMemory.cancelGoal")}</span>
          </button>
        ) : null}
        {canClearGoalStatus(status) && onClearGoal ? (
          <button
            className={smallActionButtonClass()}
            disabled={goalBusy}
            type="button"
            onClick={() => runGoalCommand(onClearGoal)}
          >
            <Eraser size={13} />
            <span className="ml-1.5">{t("chat.shortTermMemory.clearGoal")}</span>
          </button>
        ) : null}
        {canCreateGoal ? (
          <button
            className={smallActionButtonClass()}
            disabled={goalBusy}
            type="button"
            onClick={startGoalCreate}
          >
            <Plus size={13} />
            <span className="ml-1.5">{t("chat.shortTermMemory.newGoal")}</span>
          </button>
        ) : null}
      </div>
    );
  }

  return (
    <div className="space-y-5 text-sm">
      <ContextBlock
        title={t("chat.shortTermMemory.goal")}
        actions={goal ? (
          <div className="flex items-center gap-1.5">
            <Badge className="h-5 rounded px-1.5 text-[11px]" tone={goalStatusTone(goal.goal.status)}>
              {goalStatusLabel}
            </Badge>
            {canEditGoal && !editingGoal ? (
              <button
                className={smallActionButtonClass()}
                disabled={goalBusy}
                type="button"
                onClick={startGoalEdit}
              >
                <Pencil size={13} />
                <span className="ml-1.5">{t("chat.shortTermMemory.editGoal")}</span>
              </button>
            ) : null}
          </div>
        ) : null}
      >
        {hasGoalLoadState ? (
          <QueryStatusLine
            errorText={t("chat.shortTermMemory.loadFailed")}
            loadingText={t("chat.shortTermMemory.loadingGoal")}
            retryText={t("chat.shortTermMemory.retry")}
            status={goalStatus}
          />
        ) : showCreateGoalForm ? (
          renderGoalForm("create")
        ) : goal ? (
          editingGoal ? (
            renderGoalForm("edit")
          ) : (
            <div className="space-y-3">
              <div className="max-h-32 overflow-y-auto break-words pr-2 text-sm leading-5 text-[var(--color-on-surface)]">
                {goal.goal.objectiveMarkdown}
              </div>
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-[var(--color-on-surface-variant)]">
                <span>{goalVerdictLabel(goal.goal.lastVerdict, t)}</span>
                <span>{t("chat.shortTermMemory.goalRunCount", { used: goal.goal.runsUsed, max: goal.goal.maxRuns })}</span>
                {goal.goal.lastRunCode ? (
                  <span className="font-mono">{truncateLabel(goal.goal.lastRunCode, 18)}</span>
                ) : null}
              </div>
              {goal.goal.status === "BLOCKED" && goal.goal.blockedReason ? (
                <GoalDetailBlock title={t("chat.shortTermMemory.blockedReason")} tone="warning">
                  {goal.goal.blockedReason}
                </GoalDetailBlock>
              ) : null}
              {goal.goal.status === "BLOCKED" && goal.goal.requiredUserInput ? (
                <GoalDetailBlock title={t("chat.shortTermMemory.requiredUserInput")} tone="warning">
                  {goal.goal.requiredUserInput}
                </GoalDetailBlock>
              ) : null}
              {goal.goal.status !== "BLOCKED" && goal.goal.lastReason ? (
                <GoalDetailBlock title={t("chat.shortTermMemory.judgeReason")}>
                  {goal.goal.lastReason}
                </GoalDetailBlock>
              ) : null}
              {renderGoalActionButtons()}
            </div>
          )
        ) : (
          <p className="text-xs text-[var(--color-on-surface-variant)]">
            {t("chat.shortTermMemory.noGoal")}
          </p>
        )}
      </ContextBlock>

      <ContextBlock
        title={t("chat.shortTermMemory.todo")}
        actions={activeTodos.length ? (
          <Badge className="h-5 rounded px-1.5 text-[11px]" tone="secondary">
            {t("chat.shortTermMemory.activeTodoCount", { count: activeTodos.length })}
          </Badge>
        ) : null}
      >
        {hasTodoLoadState ? (
          <QueryStatusLine
            errorText={t("chat.shortTermMemory.loadFailed")}
            loadingText={t("chat.shortTermMemory.loadingTodos")}
            retryText={t("chat.shortTermMemory.retry")}
            status={todosStatus}
          />
        ) : activeTodos.length ? (
          <div className="space-y-1">
            {activeTodos.map((todo) => (
              <div
                key={todo.todoCode}
                className="grid min-w-0 grid-cols-[0.5rem_minmax(0,1fr)_auto_auto] items-center gap-2 py-1 text-xs"
                title={todo.content}
              >
                <span className={cn("size-2 rounded-full", todoStatusDotClass(todo.status))} aria-hidden="true" />
                <span className="min-w-0 truncate text-sm text-[var(--color-on-surface)]">{todo.content}</span>
                <Badge className="h-5 rounded px-1.5 text-[11px]" tone={todoStatusTone(todo.status)}>
                  {t(`chat.shortTermMemory.todoStatuses.${todo.status}`)}
                </Badge>
                <Badge className="h-5 rounded px-1.5 text-[11px]" tone={todoPriorityTone(todo.priority)}>
                  {t(`chat.shortTermMemory.todoPriorities.${todo.priority}`)}
                </Badge>
              </div>
            ))}
            {foldedTodoCount ? (
              <div className="border-t border-[var(--color-outline-variant)] pt-2 text-xs text-[var(--color-on-surface-variant)]">
                {t("chat.shortTermMemory.foldedTodoCount", { count: foldedTodoCount })}
              </div>
            ) : null}
          </div>
        ) : (
          <div className="space-y-2 text-xs text-[var(--color-on-surface-variant)]">
            <p>{t("chat.shortTermMemory.emptyTodos")}</p>
            {foldedTodoCount ? <p>{t("chat.shortTermMemory.foldedTodoCount", { count: foldedTodoCount })}</p> : null}
          </div>
        )}
      </ContextBlock>
    </div>
  );
}

function RightContextSidebar({
  events,
  goal,
  latestCompaction,
  latestCompactionStatus,
  latestRequestEstimate,
  latestUsageAudit,
  messages,
  model,
  onCreateCompaction,
  compactionBusy,
  selectedModel,
  session,
  todos,
}: {
  events: ChatAguiEvent[];
  goal?: ChatSessionGoalView | null;
  latestCompaction?: ChatSessionCompaction | null;
  latestCompactionStatus?: QueryStatus;
  latestRequestEstimate?: ChatTokenEstimateSnapshot | null;
  latestUsageAudit?: ChatUsageAuditSnapshot | null;
  messages: ChatMessage[];
  model?: ModelRecord | null;
  onCreateCompaction?: () => void;
  compactionBusy?: boolean;
  selectedModel?: ChatModelOption | null;
  session?: ChatSession;
  todos?: ChatSessionTodo[];
}) {
  const { t } = useTranslation("pages");
  const modelLabel = (
    contextValue(model?.displayName)
    ?? contextValue(model?.modelName)
    ?? contextValue(selectedModel?.displayName)
    ?? contextValue(selectedModel?.modelName)
    ?? contextValue(latestEventValue(events, "modelName"))
    ?? contextValue(latestEventValue(events, "model_name"))
    ?? "-"
  );
  const usage = chatTokenUsage(messages);
  const usageAudit = latestUsageAudit ?? latestUsageAuditFromEvents(events);
  const requestEstimate = latestRequestEstimate ?? usageAudit?.requestEstimate ?? latestUsageEstimateFromEvents(events);
  const budgetBreakdown = latestContextBudgetBreakdownFromEvents(events);
  const fallbackWindowLimit = contextWindowLimitFromSources(model, selectedModel, events);
  const contextBlocks = buildContextMemoryBlocks({
    breakdown: budgetBreakdown,
    goal,
    latestCompaction,
    requestEstimate,
    todos,
    translate: (key, options) => t(key, options),
  });
  return (
    <section className="space-y-5 text-sm">
      <ContextOverviewPanel
        breakdown={budgetBreakdown}
        fallbackWindowLimit={fallbackWindowLimit}
        modelLabel={modelLabel}
        requestEstimate={requestEstimate}
      />
      <ContextStructurePanel
        blocks={contextBlocks}
        compactionBusy={compactionBusy || latestCompactionStatus?.loading}
        onCreateCompaction={onCreateCompaction}
        session={session}
      />
      {latestCompactionStatus?.error ? (
        <QueryStatusLine
          errorText={t("chat.shortTermMemory.loadFailed")}
          loadingText={t("chat.shortTermMemory.loadingCheckpoint")}
          retryText={t("chat.shortTermMemory.retry")}
          status={latestCompactionStatus}
        />
      ) : null}
      <ProviderUsagePanel fallbackSessionUsage={usage} usageAudit={usageAudit} />
    </section>
  );
}

function ContextBlock({
  actions,
  children,
  className,
  title,
}: {
  actions?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  title: React.ReactNode;
}) {
  return (
    <section className={cn("space-y-2 border-b border-[var(--color-outline-variant)] pb-4 last:border-b-0", className)}>
      <div className="flex min-w-0 items-center justify-between gap-2">
        <h3 className="truncate text-xs font-semibold uppercase text-[var(--color-on-surface-variant)]">{title}</h3>
        {actions ? <div className="shrink-0">{actions}</div> : null}
      </div>
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
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const selectedModel = models.find((model) => model.optionCode === selectedModelCode) ?? null;
  const filteredModels = useMemo(
    () => models.filter((model) => fuzzyModelMatch(searchQuery, model)),
    [models, searchQuery],
  );
  const disabled = loading || models.length === 0;
  const selectedModelLabel = loading ? "Loading models..." : selectedModel ? selectedModel.displayName : emptyLabel;

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
      <DropdownMenu open={open} onOpenChange={setOpen}>
        <DropdownMenuTrigger
          aria-label="Select chat model"
          className="inline-flex h-8 w-full min-w-0 items-center gap-2 rounded-md border-0 bg-transparent px-2 text-xs text-[var(--color-on-surface-variant)] shadow-none transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
          disabled={disabled}
          title={selectedModelLabel}
        >
          <Sparkles aria-hidden="true" size={14} />
          <span className="truncate">{selectedModelLabel}</span>
          <ChevronDown aria-hidden="true" size={14} />
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="start"
          className="max-h-72 min-w-[180px] max-w-[260px] overflow-y-auto overscroll-contain p-1.5"
          side="top"
          sideOffset={8}
        >
          <div className="sticky top-0 z-10 bg-popover pb-1.5">
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
            <DropdownMenuItem
              className={cn(
                "w-full items-start px-3 py-2",
                model.optionCode === selectedModelCode && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
              )}
              key={model.optionCode}
              title={model.displayName}
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
              }}
            >
              {model.optionCode === selectedModelCode ? <Check aria-hidden="true" className="mt-0.5 size-4 shrink-0" /> : <span className="size-4 shrink-0" />}
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
            </DropdownMenuItem>
          )) : (
            <p className="px-3 py-2 text-xs text-[var(--color-on-surface-variant)]">
              No models match "{searchQuery.trim()}".
            </p>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
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
  const selectedAgent = agents.find((agent) => agent.agentCode === selectedAgentCode) ?? null;
  const disabled = loading || agents.length === 0 || Boolean(locked);

  return (
    <div className="relative min-w-0">
      <DropdownMenu open={open} onOpenChange={setOpen}>
        <DropdownMenuTrigger
          aria-label="Select chat agent"
          className="inline-flex h-8 w-full min-w-0 items-center gap-2 rounded-md border-0 bg-transparent px-2 text-xs text-[var(--color-on-surface-variant)] shadow-none transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
          disabled={disabled}
          title={locked ? "Agent is locked after the first chat turn in this session" : undefined}
        >
          <Bot aria-hidden="true" size={14} />
          <span className="truncate">{loading ? "Loading agents..." : selectedAgent?.name ?? "Main Agent"}</span>
          <ChevronDown aria-hidden="true" size={14} />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="max-h-72 min-w-[180px] max-w-[260px] overflow-y-auto p-1.5" side="top" sideOffset={8}>
          {agents.map((agent) => (
            <DropdownMenuItem
              className={cn(
                "w-full flex-col items-start px-3 py-2 pr-8",
                agent.agentCode === selectedAgentCode && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
              )}
              key={agent.agentCode}
              onClick={() => {
                onSelect(agent.agentCode);
              }}
            >
              {agent.agentCode === selectedAgentCode ? <Check aria-hidden="true" className="absolute right-2 top-2 size-4" /> : null}
              <span className="block min-w-0 truncate font-medium">{agent.name}</span>
              <span className="mt-0.5 block min-w-0 truncate text-[11px] text-[var(--color-on-surface-variant)]">
                {agent.agentCode}
              </span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
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
            ? "bg-[var(--color-surface-container-high)]"
            : "hover:bg-[var(--color-surface-container-high)]",
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
      data-testid="chat-session-sidebar"
      className={cn(
        "relative h-full shrink-0 overflow-visible bg-[var(--color-surface-container-lowest)] shadow-[10px_0_28px_rgba(15,23,42,0.04)] transition-[width] duration-200",
        collapsed ? "w-[64px]" : "w-[286px]",
      )}
    >
      {collapsed ? (
        <div className="flex h-full flex-col overflow-y-auto py-3">
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
      <DraggleLine
        ariaLabel={collapsed ? "Expand session history" : "Collapse session history"}
        onClick={onCollapseToggle}
        className="absolute right-0 top-0 z-10 h-full"
      />
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
              {nested ? null : <Badge tone={complete ? "info" : "success"}>{complete ? "done" : "running"}</Badge>}
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
              <Badge tone="info">{type}</Badge>
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
  const openRightSidebar = useOpenRightSidebar();
  const { t } = useTranslation("pages");
  const { session: authSession } = useAuthSession();
  const { preferences: aguiDebugPreferences } = useAguiDebugPreferences();
  const { preferences: aguiWorkReplayPreferences } = useAguiWorkReplayPreferences();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const [activeChatDataRefreshKey, setActiveChatDataRefreshKey] = useState(0);
  const [activeChatReplayRefreshKey, setActiveChatReplayRefreshKey] = useState(0);
  const [activeRunRefreshKey, setActiveRunRefreshKey] = useState(0);
  const [shortTermMemoryRefreshKey, setShortTermMemoryRefreshKey] = useState(0);
  const groups = useChatSessionGroupList(activeWorkspaceCode, activeChatDataRefreshKey);
  const [sessionGroups, setSessionGroups] = useState<ChatSessionGroup[]>([]);
  const [activeSessionCode, setActiveSessionCode] = useState("");
  const [activeSourceTab, setActiveSourceTab] = useState<ChatSessionSourceTab>(() => (
    requestedSourceTab ?? defaultChatSessionSourceTab
  ));
  const [userSidebarCollapsed, setUserSidebarCollapsed] = useState(false);
  const [autoSidebarCollapsed, setAutoSidebarCollapsed] = useState(false);
  const sidebarCollapsed = userSidebarCollapsed || autoSidebarCollapsed;
  const mainRef = useRef<HTMLDivElement>(null);
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
  const activeRunState = useChatActiveRunState(
    activeSession?.sessionCode ?? "",
    activeRunRefreshKey,
  );
  const shortTermMemoryTodosState = useChatSessionTodosState(activeSession?.sessionCode ?? "", shortTermMemoryRefreshKey);
  const latestCompactionState = useChatLatestCompactionState(activeSession?.sessionCode ?? "", shortTermMemoryRefreshKey);
  const activeGoalState = useChatSessionLatestGoalState(activeSession?.sessionCode ?? "", shortTermMemoryRefreshKey);
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
  const aguiEvents = useChatAguiEvents(activeSession?.sessionCode ?? "", activeChatDataRefreshKey + shortTermMemoryRefreshKey);
  const aguiProtocolEvents = useChatAguiProtocolEvents(activeSession?.sessionCode ?? "", activeChatReplayRefreshKey);
  const [visibleMessages, setVisibleMessages] = useState<ChatMessage[]>([]);
  const [liveAguiEventsBySessionCode, setLiveAguiEventsBySessionCode] = useState<Record<string, ChatAguiStreamEvent[]>>({});
  const [liveAguiAnchorMessageCodesBySessionCode, setLiveAguiAnchorMessageCodesBySessionCode] = useState<Record<string, Record<string, string>>>({});
  const [latestRequestEstimatesBySessionCode, setLatestRequestEstimatesBySessionCode] = useState<Record<string, ChatTokenEstimateSnapshot>>({});
  const [latestUsageAuditsBySessionCode, setLatestUsageAuditsBySessionCode] = useState<Record<string, ChatUsageAuditSnapshot>>({});
  const [sending, setSending] = useState(false);
  const [steering, setSteering] = useState(false);
  const [activeRunCode, setActiveRunCode] = useState("");
  const [activeRunSupportsSteer, setActiveRunSupportsSteer] = useState<boolean | null>(null);
  const [queuedSubmissions, setQueuedSubmissions] = useState<ChatQueuedSubmission[]>([]);
  const [creatingSession, setCreatingSession] = useState(false);
  const [chatModels, setChatModels] = useState<ModelRecord[]>([]);
  const [selectedModelCode, setSelectedModelCode] = useState("");
  const [selectedReasoningEffort, setSelectedReasoningEffort] = useState<ReasoningEffort | null>(null);
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
  const [pendingAttachments, setPendingAttachments] = useState<ChatMessageAttachment[]>([]);
  const [attachmentUploadPending, setAttachmentUploadPending] = useState(false);
  const activeSessionCodeRef = useRef("");
  const modelSelectionSessionRef = useRef("");
  const streamOwnershipRegistryRef = useRef(new ChatStreamOwnershipRegistry());
  const detachedStreamControllersRef = useRef(new WeakSet<AbortController>());
  const stopRequestedStreamControllersRef = useRef(new WeakSet<AbortController>());
  const steerAbortControllerRef = useRef<AbortController | null>(null);
  const stopAbortControllerRef = useRef<AbortController | null>(null);
  const sendingRef = useRef(false);
  const steeringRef = useRef(false);
  const steeringSessionCodeRef = useRef("");
  const stoppingRunRef = useRef(false);
  const activeRunCodeRef = useRef("");
  const activeRunSupportsSteerRef = useRef<boolean | null>(null);
  const autoDrainEpochRef = useRef(0);
  const runningSessionCodeRef = useRef("");
  const queuedSubmissionsRef = useRef<ChatQueuedSubmission[]>([]);
  const queuedSubmissionSequenceRef = useRef(0);
  const invalidatedQueueSessionCodesRef = useRef<Set<string>>(new Set());
  const observedRecoveredRunCodesRef = useRef<Map<string, string>>(new Map());
  const drainedRecoveredRunKeysRef = useRef<Set<string>>(new Set());
  const requestedSessionCodeSyncedRef = useRef("");
  const autoStickToBottomRef = useRef(true);
  const messageScrollRef = useRef<HTMLElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const attachmentFileInputRef = useRef<HTMLInputElement | null>(null);
  const scrollFrameRef = useRef<number | null>(null);
  const sessionTitleRefreshTimersRef = useRef<number[]>([]);
  const visibleSessionCodeRef = useRef("");
  const liveAssistantDraftRef = useRef("");
  const liveAguiObservedRef = useRef(false);
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
  const selectedReasoningEffortOptions = useMemo(() => {
    const supported = new Set(selectedMainAgentModel?.supportedReasoningEfforts ?? []);
    return reasoningEffortOptions
      .filter((option) => supported.has(option.value))
      .map((option) => ({
        ...option,
        label: t(`common:reasoningEffort.values.${option.value}`, option.label),
      }));
  }, [selectedMainAgentModel?.supportedReasoningEfforts, t]);
  const selectedModelDefaultReasoningEffort = selectedMainAgentModel?.defaultReasoningEffort ?? null;
  const selectedReasoningEffortDefaultLabel = selectedModelDefaultReasoningEffort
    ? t(
      `common:reasoningEffort.values.${selectedModelDefaultReasoningEffort}`,
      formatReasoningEffort(selectedModelDefaultReasoningEffort),
    )
    : t("common:reasoningEffort.chatAutomatic", "Auto");
  const selectedReasoningEffortLabel = selectedReasoningEffort
    ? selectedReasoningEffortOptions.find((option) => option.value === selectedReasoningEffort)?.label
      ?? formatReasoningEffort(selectedReasoningEffort)
    : selectedReasoningEffortDefaultLabel;
  const selectedReasoningEffortControlValue = selectedReasoningEffort
    ?? selectedModelDefaultReasoningEffort
    ?? modelDefaultReasoningEffortValue;
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
  const { createInspectionAttachmentUpload } = useInspection();

  function setSendingState(value: boolean) {
    sendingRef.current = value;
    setSending(value);
  }

  function scheduleSessionTitleRefresh(session: ChatSession) {
    const defaultTitle = session.title.trim().toLocaleLowerCase();
    if (defaultTitle !== "new chat" && defaultTitle !== "main agent") return;
    [500, 1500, 3000, 6000, 12000].forEach((delay) => {
      const timer = window.setTimeout(() => {
        sessionTitleRefreshTimersRef.current = sessionTitleRefreshTimersRef.current.filter(
          (candidate) => candidate !== timer,
        );
        setActiveChatDataRefreshKey((current) => current + 1);
      }, delay);
      sessionTitleRefreshTimersRef.current.push(timer);
    });
  }

  useEffect(() => () => {
    sessionTitleRefreshTimersRef.current.forEach((timer) => window.clearTimeout(timer));
    sessionTitleRefreshTimersRef.current = [];
  }, []);

  function replaceQueuedSubmissions(next: ChatQueuedSubmission[]) {
    queuedSubmissionsRef.current = next;
    setQueuedSubmissions(next);
  }

  function enqueueSubmission(
    session: ChatSession,
    input: SendChatMessageInput,
    position: "front" | "back" = "back",
  ) {
    if (invalidatedQueueSessionCodesRef.current.has(session.sessionCode)) {
      return null;
    }
    queuedSubmissionSequenceRef.current += 1;
    const submission: ChatQueuedSubmission = {
      id: `QUEUED-${Date.now()}-${queuedSubmissionSequenceRef.current}`,
      input,
      sessionCode: session.sessionCode,
    };
    replaceQueuedSubmissions(
      position === "front"
        ? [submission, ...queuedSubmissionsRef.current]
        : enqueueChatSubmission(queuedSubmissionsRef.current, submission),
    );
    return submission;
  }

  function takeNextQueuedSubmission(sessionCode: string) {
    if (invalidatedQueueSessionCodesRef.current.has(sessionCode)) return null;
    const result = takeNextQueuedChatSubmission(
      queuedSubmissionsRef.current,
      sessionCode,
    );
    replaceQueuedSubmissions(result.remaining);
    return result.submission;
  }

  function invalidateQueuedSession(sessionCode: string) {
    invalidatedQueueSessionCodesRef.current.add(sessionCode);
    replaceQueuedSubmissions(
      queuedSubmissionsRef.current.filter((submission) => (
        submission.sessionCode !== sessionCode
      )),
    );
  }

  function createSendInput(
    session: ChatSession,
    content: string,
    attachments: ChatMessageAttachment[],
  ): SendChatMessageInput | null {
    if (!selectedModel) return null;
    const input: SendChatMessageInput = {
      agentCode: effectiveAgentCode,
      appCode: session.appCode,
      content,
      taskCode: session.taskCode,
      workspaceCode: session.workspaceCode,
    };
    if (attachments.length) input.attachments = attachments;
    if (selectedModel.source === "system-agent") {
      input.systemModelId = selectedModel.modelName;
    } else {
      input.modelCode = selectedModel.modelCode;
      input.reasoningEffort = selectedReasoningEffort;
    }
    return input;
  }

  async function uploadAttachmentForActiveSession(
    file: File,
    options: Omit<Parameters<typeof uploadChatSessionAttachment>[1], "file"> = {},
  ): Promise<ChatMessageAttachment | null> {
    const targetSession = activeSession;
    if (!targetSession) {
      toast.warning("Create or select a session before attaching a file", { title: "Session required" });
      return null;
    }

    setAttachmentUploadPending(true);
    try {
      const attachment = await uploadChatSessionAttachment(targetSession, {
        ...options,
        file,
      });
      return activeSessionCodeRef.current === targetSession.sessionCode
        ? attachment
        : null;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Chat attachment upload failed", {
        title: "Attachment upload failed",
      });
      return null;
    } finally {
      setAttachmentUploadPending(false);
    }
  }

  async function handleAttachmentFileInputChange(
    event: ChangeEvent<HTMLInputElement>,
  ) {
    const file = event.currentTarget.files?.item(0);
    event.currentTarget.value = "";
    if (!file) return;
    const attachment = await uploadAttachmentForActiveSession(file, {
      kind: file.type.startsWith("image/") ? "image" : "file",
      summary: file.name,
    });
    if (!attachment) return;
    setPendingAttachments((current) => [...current, attachment]);
  }

  async function handleInspectionConfirm(result: TargetPageFrameInspectionResult) {
    try {
      const inspection = await createInspectionAttachmentUpload(result);
      const attachment = await uploadAttachmentForActiveSession(inspection.file, {
        kind: inspection.kind,
        metadata: inspection.metadata,
        summary: inspection.summary,
      });
      if (!attachment) return;

      const content = inspectionMessageContent(result);
      if (inspectionSubmitMode(result) === "send" && selectedModel) {
        await handleSend(content, [attachment]);
        return;
      }
      if (content) setComposer(content);
      setPendingAttachments((current) => [...current, attachment]);
      if (inspectionSubmitMode(result) === "send" && !selectedModel) {
        toast.warning("Select a model before sending the page annotation", { title: "Model required" });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Page annotation attachment failed", {
        title: "Attachment upload failed",
      });
    }
  }

  const replaceChatSearchParams = useCallback((update: (next: URLSearchParams) => void) => {
    const currentPathname = typeof window === "undefined" ? "/chats" : window.location.pathname;
    const currentSearch = typeof window === "undefined"
      ? currentSearchText
      : window.location.search.replace(/^\?/, "");
    if (!isChatManagementPathname(currentPathname)) {
      return;
    }
    const next = new URLSearchParams(currentSearch);
    update(next);
    const nextSearch = next.toString();
    if (nextSearch === currentSearch) {
      if (currentSearchText !== nextSearch) setCurrentSearchText(nextSearch);
      return;
    }
    setCurrentSearchText(nextSearch);
    navigate(`${currentPathname}${nextSearch ? `?${nextSearch}` : ""}`, { replace: true });
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
    setSelectedReasoningEffort(null);
  }, [activeSession?.sessionCode, selectedModel?.optionCode, selectedMainAgentModel?.updatedAt]);

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
    const owner = streamOwnershipRegistryRef.current.current;
    if (owner) {
      autoDrainEpochRef.current += 1;
      detachedStreamControllersRef.current.add(owner.controller);
      streamOwnershipRegistryRef.current.release(owner);
      sendingRef.current = false;
      runningSessionCodeRef.current = "";
      owner.controller.abort();
    }
    steerAbortControllerRef.current?.abort();
  }, []);

  useEffect(() => {
    const owner = streamOwnershipRegistryRef.current.current;
    if (
      owner
      && owner.sessionCode !== (activeSession?.sessionCode ?? "")
    ) {
      detachLocalRunObserver(owner.sessionCode);
    }
  }, [activeSession?.sessionCode]);

  useEffect(() => {
    setPendingAttachments([]);
  }, [activeSession?.sessionCode]);

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
  const replayActiveRunHint = useMemo(
    () => resolveLatestOpenChatReplayRun(
      aguiProtocolEvents,
      activeSession?.sessionCode ?? "",
    ),
    [activeSession?.sessionCode, aguiProtocolEvents],
  );
  const activeRunQueryMatchesSession = Boolean(
    activeSession?.sessionCode
    && activeRunState.data.sessionCode === activeSession.sessionCode,
  );
  const authoritativeActiveRun = activeRunQueryMatchesSession
    ? activeRunState.data.activeRun
    : null;
  const replayActiveRunLoadingHint = (
    activeRunState.loading
    || Boolean(activeRunState.error)
    || !activeRunQueryMatchesSession
  )
    ? replayActiveRunHint
    : null;
  const runUiState = resolveChatRunUiState({
    authoritativeRun: authoritativeActiveRun,
    localRunActive: canAbortCurrentSession,
    localRunCode: activeRunCode,
    localRunSupportsSteer: activeRunSupportsSteer,
    replayRunHint: replayActiveRunLoadingHint,
  });
  const currentSessionRunActive = runUiState.active;
  const effectiveActiveRunCode = runUiState.runCode;
  const canSteerCurrentRun = runUiState.canSteer;
  const canStopCurrentRun = runUiState.canStop;
  const runningSubmitModes = runUiState.runningSubmitModes;
  const visibleQueuedSubmissions = useMemo(
    () => queuedSubmissions.filter((submission) => (
      submission.sessionCode === activeSession?.sessionCode
    )),
    [activeSession?.sessionCode, queuedSubmissions],
  );
  const hasQueuedSubmissionForActiveSession = visibleQueuedSubmissions.length > 0;
  const queueDisabled = Boolean(
    !activeSession
    || !selectedModel
    || (!composer.trim() && !pendingAttachments.length)
    || creatingSession,
  );
  const steerDisabled = queueDisabled || steering || !canSteerCurrentRun;
  const hasOpenReplayRun = Boolean(replayActiveRunHint);
  const shouldPollActiveChatData = Boolean(
    activeSession?.sessionCode
    && !(sending && runningSessionCode === activeSession.sessionCode)
    && (
      isRecentPendingUserTurn(visibleMessages)
      || hasOpenReplayRun
      || hasQueuedSubmissionForActiveSession
    )
  );
  const shouldPollActiveChatReplay = shouldPollChatRunRecovery({
    currentRunActive: currentSessionRunActive,
    hasQueuedSubmission: hasQueuedSubmissionForActiveSession,
    sessionCode: activeSession?.sessionCode ?? "",
    shouldPollData: shouldPollActiveChatData,
  });
  const handleCreateCompactionForRightPanel = useStableCallback(handleCreateCompaction);
  const handleCancelGoalForRightPanel = useStableCallback(handleCancelGoal);
  const handleClearGoalForRightPanel = useStableCallback(handleClearGoal);
  const handleCreateGoalForRightPanel = useStableCallback(handleCreateGoal);
  const handlePauseGoalForRightPanel = useStableCallback(handlePauseGoal);
  const handleResumeGoalForRightPanel = useStableCallback(handleResumeGoal);
  const handleUpdateGoalForRightPanel = useStableCallback(handleUpdateGoal);
  const workspaceGoalRunning = Boolean(
    activeSession?.sessionCode
    && activeGoal?.goal.status === "ACTIVE"
    && (
      (sending && runningSessionCode === activeSession.sessionCode)
      || hasOpenReplayRun
    ),
  );

  const rightPanel = useMemo(() => {
    if (!hasActiveSession) return undefined;
    const tabs: EditableTab[] = [
      {
        key: "workspace",
        label: (
          <>
            <WorkspaceTabState goal={activeGoal} running={workspaceGoalRunning} />
            <span>{t("chat.shortTermMemory.workspace")}</span>
          </>
        ),
        closable: false,
        children: (
          <div className="min-h-0 overflow-y-auto">
            <div className="m-4">
              <RightWorkspaceSidebar
                goal={activeGoal}
                goalBusy={Boolean(activeSession?.sessionCode && goalActionSessionCode === activeSession.sessionCode)}
                goalStatus={goalStatus}
                onCancelGoal={handleCancelGoalForRightPanel}
                onClearGoal={handleClearGoalForRightPanel}
                onCreateGoal={handleCreateGoalForRightPanel}
                onPauseGoal={handlePauseGoalForRightPanel}
                onResumeGoal={handleResumeGoalForRightPanel}
                onUpdateGoal={handleUpdateGoalForRightPanel}
                sessionCode={activeSession?.sessionCode}
                todos={shortTermMemoryTodos}
                todosStatus={shortTermMemoryTodosStatus}
              />
            </div>
          </div>
        ),
      },
      {
        key: "context",
        label: t("chat.shortTermMemory.context"),
        closable: false,
        children: (
          <div className="min-h-0 overflow-y-auto">
            <div className="m-4">
              <RightContextSidebar
                events={aguiEvents}
                goal={activeGoal}
                latestCompaction={latestCompaction}
                latestCompactionStatus={latestCompactionStatus}
                latestRequestEstimate={activeSession ? latestRequestEstimatesBySessionCode[activeSession.sessionCode] ?? null : null}
                latestUsageAudit={activeSession ? latestUsageAuditsBySessionCode[activeSession.sessionCode] ?? null : null}
                messages={visibleMessages}
                model={selectedMainAgentModel}
                compactionBusy={Boolean(activeSession?.sessionCode && compactingSessionCode === activeSession.sessionCode)}
                onCreateCompaction={handleCreateCompactionForRightPanel}
                selectedModel={selectedModel}
                session={activeSession}
                todos={shortTermMemoryTodos}
              />
            </div>
          </div>
        ),
      },
      {
        key: "page",
        label: t("chat.shortTermMemory.page"),
        closable: false,
        children: (
          <div className="min-h-0 h-full">
            <PageView
              url="https://www.baidu.com"
              onInspectionConfirm={(result) => { void handleInspectionConfirm(result); }}
            />
          </div>
        ),
      },
    ];

    return (
      <EditableTabs
        key={activeSession?.sessionCode}
        className="h-full min-h-0"
        defaultActiveKey="workspace"
        hideAdd
        items={tabs}
      />
    );
  }, [
    activeSession,
    activeGoal,
    aguiEvents,
    compactingSessionCode,
    goalActionSessionCode,
    goalStatus,
    hasActiveSession,
    handleCancelGoalForRightPanel,
    handleClearGoalForRightPanel,
    handleCreateCompactionForRightPanel,
    handleCreateGoalForRightPanel,
    handlePauseGoalForRightPanel,
    handleResumeGoalForRightPanel,
    handleInspectionConfirm,
    handleUpdateGoalForRightPanel,
    latestCompaction,
    latestCompactionStatus,
    latestRequestEstimatesBySessionCode,
    latestUsageAuditsBySessionCode,
    selectedMainAgentModel,
    selectedModel,
    shortTermMemoryTodos,
    shortTermMemoryTodosStatus,
    t,
    visibleMessages,
    workspaceGoalRunning,
  ]);
  const transcriptDebugOptions = useMemo(() => ({
    debugEventTypes: resolveAguiDebugEventTypes(aguiDebugPreferences),
    recentTurnLimit: aguiWorkReplayPreferences.recentTurnLimit,
  }), [aguiDebugPreferences, aguiWorkReplayPreferences.recentTurnLimit]);
  const downloadPendingAttachment = useCallback(
    (attachment: ChatMessageAttachment) => {
      if (!activeSession) return Promise.reject(new Error("No active chat session"));
      return downloadChatSessionAttachment(activeSession, attachment).then((response) => response.blob);
    },
    [activeSession],
  );
  const transcriptAttachmentHeader = pendingAttachments.length ? (
    <div className="flex max-w-full flex-wrap gap-2">
      {pendingAttachments.map((attachment) => (
        <ChatAttachmentChip
          attachment={attachment}
          downloadAttachment={downloadPendingAttachment}
          key={attachment.id}
          onRemove={() => {
            setPendingAttachments((current) => current.filter((item) => item.id !== attachment.id));
          }}
        />
      ))}
    </div>
  ) : null;
  const transcriptFooterAddon = useMemo(() => (
    <div className="flex w-full min-w-0 max-w-full flex-col items-start gap-2">
      <div className="flex w-full min-w-0 max-w-full flex-wrap items-center gap-1">
        <input
          className="hidden"
          ref={attachmentFileInputRef}
          type="file"
          onChange={(event) => { void handleAttachmentFileInputChange(event); }}
        />
        <Button
          aria-label="Upload attachment"
          disabled={!activeSession || attachmentUploadPending}
          size="icon"
          title={attachmentUploadPending ? "Uploading…" : "Upload attachment"}
          type="button"
          variant="ghost"
          onClick={() => attachmentFileInputRef.current?.click()}
        >
          <Paperclip aria-hidden="true" />
        </Button>
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
          className={cn("min-w-0 max-w-full")}
        >
          <ChatModelSelector
            emptyLabel={modelEmptyLabel}
            loading={modelsLoading}
            models={agentScopedModels}
            selectedModelCode={selectedModelCode}
            onSelect={handleModelSelect}
          />
        </div>
        {selectedReasoningEffortOptions.length ? (
          <DropdownMenu>
            <DropdownMenuTrigger
              aria-label={t("common:reasoningEffort.chatLabel", "Reasoning effort")}
              className="inline-flex h-8 flex-none items-center gap-2 rounded-md border-0 bg-transparent px-2 text-xs text-[var(--color-on-surface-variant)] shadow-none transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
            >
              <BrainCircuit aria-hidden="true" className="size-3.5 shrink-0" />
              <span className="min-w-0 truncate">{selectedReasoningEffortLabel}</span>
              <ChevronDown aria-hidden="true" className="size-3.5 shrink-0" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" side="top" sideOffset={8}>
              {selectedModelDefaultReasoningEffort ? null : (
                <DropdownMenuItem
                  className="relative pr-8"
                  onClick={() => setSelectedReasoningEffort(null)}
                >
                  {selectedReasoningEffortControlValue === modelDefaultReasoningEffortValue ? <Check aria-hidden="true" className="absolute right-2 size-4" /> : null}
                  {selectedReasoningEffortDefaultLabel}
                </DropdownMenuItem>
              )}
              {selectedReasoningEffortOptions.map((option) => {
                const selected = selectedReasoningEffortControlValue === option.value;
                return (
                  <DropdownMenuItem
                    className="relative pr-8"
                    key={option.value}
                    onClick={() => setSelectedReasoningEffort(
                      option.value === selectedModelDefaultReasoningEffort ? null : option.value as ReasoningEffort,
                    )}
                  >
                    {selected ? <Check aria-hidden="true" className="absolute right-2 size-4" /> : null}
                    {option.label}
                  </DropdownMenuItem>
                );
              })}
            </DropdownMenuContent>
          </DropdownMenu>
        ) : null}
      </div>
    </div>
  ), [
    agentScopedModels,
    agentSelectionLocked,
    agentsLoading,
    activeSession,
    attachmentUploadPending,
    chatAgents,
    handleAttachmentFileInputChange,
    modelEmptyLabel,
    modelsLoading,
    selectedAgentCode,
    selectedModelCode,
    selectedModelDefaultReasoningEffort,
    selectedReasoningEffort,
    selectedReasoningEffortControlValue,
    selectedReasoningEffortDefaultLabel,
    selectedReasoningEffortLabel,
    selectedReasoningEffortOptions,
    t,
  ]);
  const formatAnnotationCount = useCallback(
    (count: number) => t("chat.attachments.annotationCount", { count }),
    [t],
  );

  useConsoleShellConfig({ rightPanel });

  useEffect(() => {
    if (!shouldPollActiveChatReplay) return;
    const intervalId = window.setInterval(() => {
      setActiveChatReplayRefreshKey((current) => current + 1);
      setActiveRunRefreshKey((current) => current + 1);
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
    if (!activeSession || canAbortCurrentSession) return;
    const sessionCode = activeSession.sessionCode;
    const observedRemoteRun = authoritativeActiveRun ?? replayActiveRunHint;
    if (observedRemoteRun) {
      observedRecoveredRunCodesRef.current.set(
        sessionCode,
        observedRemoteRun.runCode,
      );
      return;
    }
    const observedRunCode = observedRecoveredRunCodesRef.current.get(sessionCode)
      ?? (hasQueuedSubmissionForActiveSession
        ? latestTerminalChatRunCode(aguiProtocolEvents, sessionCode)
        : "");
    if (
      !observedRunCode
      || !isChatReplayRunTerminal(
        aguiProtocolEvents,
        sessionCode,
        observedRunCode,
      )
    ) return;
    observedRecoveredRunCodesRef.current.delete(sessionCode);
    if (!claimChatRunTerminalDrain(
      drainedRecoveredRunKeysRef.current,
      sessionCode,
      observedRunCode,
    )) return;
    if (
      sendingRef.current
      || invalidatedQueueSessionCodesRef.current.has(sessionCode)
    ) return;
    const nextSubmission = takeNextQueuedSubmission(sessionCode);
    if (nextSubmission) {
      void runChatInput(
        activeSession,
        nextSubmission.input,
        false,
        nextSubmission,
      );
    }
  }, [
    activeSession,
    aguiProtocolEvents,
    authoritativeActiveRun,
    canAbortCurrentSession,
    hasQueuedSubmissionForActiveSession,
    replayActiveRunHint,
  ]);

  useEffect(() => {
    if (autoStickToBottomRef.current) scrollToLatestMessage("auto");
    else handleTranscriptScroll();
  }, [transcriptAguiEvents, visibleMessages]);

  function handleSessionSelect(sessionOrCode: ChatSession | string, sourceTab?: ChatSessionSourceTab) {
    const sessionCode = typeof sessionOrCode === "string" ? sessionOrCode : sessionOrCode.sessionCode;
    const selected = typeof sessionOrCode === "string" ? findSession(sessionGroups, sessionCode) : sessionOrCode;
    const currentOwner = streamOwnershipRegistryRef.current.current;
    if (currentOwner && currentOwner.sessionCode !== sessionCode) {
      detachLocalRunObserver(currentOwner.sessionCode);
    }
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
    detachLocalRunObserver();
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

    detachLocalRunObserver();
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

  function clearLocalStreamState() {
    setSendingState(false);
    runningSessionCodeRef.current = "";
    setRunningSessionCode("");
    liveAssistantDraftRef.current = "";
    liveAguiObservedRef.current = false;
    activeRunCodeRef.current = "";
    activeRunSupportsSteerRef.current = null;
    setActiveRunCode("");
    setActiveRunSupportsSteer(null);
  }

  function clearLocalStreamOwnership(owner: ChatStreamOwner) {
    if (!streamOwnershipRegistryRef.current.release(owner)) return false;
    clearLocalStreamState();
    return true;
  }

  function detachLocalRunObserver(sessionCode?: string) {
    const owner = streamOwnershipRegistryRef.current.current;
    const ownsStream = Boolean(owner && (!sessionCode || owner.sessionCode === sessionCode));
    const ownsSteer = steeringRef.current
      && (!sessionCode || steeringSessionCodeRef.current === sessionCode);
    if (!ownsStream && !ownsSteer) return;
    if (ownsStream) {
      const detachedOwner = streamOwnershipRegistryRef.current.detach(sessionCode);
      if (!detachedOwner) return;
      autoDrainEpochRef.current += 1;
      detachedStreamControllersRef.current.add(detachedOwner.controller);
      clearLocalStreamState();
      detachedOwner.controller.abort();
    }
    if (ownsSteer) steerAbortControllerRef.current?.abort();
  }

  async function handleStopRun() {
    if (stoppingRunRef.current || !activeSession || !canStopCurrentRun) return;
    const sessionCode = activeSession.sessionCode;
    const expectedRunCode = effectiveActiveRunCode;
    const currentOwner = streamOwnershipRegistryRef.current.current;
    const localOwner = currentOwner?.sessionCode === sessionCode
      ? currentOwner
      : null;
    const abortController = new AbortController();
    stopAbortControllerRef.current = abortController;
    stoppingRunRef.current = true;
    autoDrainEpochRef.current += 1;
    try {
      await stopChatRun(
        { expectedRunCode, sessionCode },
        abortController.signal,
      );
      drainedRecoveredRunKeysRef.current.add(`${sessionCode}:${expectedRunCode}`);
      observedRecoveredRunCodesRef.current.delete(sessionCode);
      if (localOwner) {
        stopRequestedStreamControllersRef.current.add(localOwner.controller);
      }
      if (localOwner && clearLocalStreamOwnership(localOwner)) {
        localOwner.controller.abort();
      }
      if (steeringSessionCodeRef.current === sessionCode) {
        steerAbortControllerRef.current?.abort();
      }
      setActiveRunRefreshKey((current) => current + 1);
      setActiveChatReplayRefreshKey((current) => current + 1);
      setActiveChatDataRefreshKey((current) => current + 1);
      toast.info("Current response has been stopped", { title: "Chat stopped" });
    } catch (error) {
      if (!isAbortError(error)) {
        toast.error(
          error instanceof Error ? error.message : "Chat stop failed",
          { title: "Chat stop failed" },
        );
      }
    } finally {
      if (stopAbortControllerRef.current === abortController) {
        stopAbortControllerRef.current = null;
      }
      stoppingRunRef.current = false;
    }
  }

  async function stopSessionRunBeforeMutation(sessionCode: string) {
    const currentOwner = streamOwnershipRegistryRef.current.current;
    const localOwner = currentOwner?.sessionCode === sessionCode
      ? currentOwner
      : null;
    const snapshot = await getChatActiveRun(sessionCode);
    const expectedRunCode = snapshot.activeRun?.runCode
      ?? (localOwner ? activeRunCodeRef.current : "");
    if (!expectedRunCode) {
      if (localOwner) {
        throw new Error("The current response is still starting; try again shortly");
      }
      return;
    }

    if (localOwner) {
      detachedStreamControllersRef.current.add(localOwner.controller);
    }
    try {
      await stopChatRun({ expectedRunCode, sessionCode });
    } catch (error) {
      if (localOwner) {
        detachedStreamControllersRef.current.delete(localOwner.controller);
      }
      throw error;
    }
    drainedRecoveredRunKeysRef.current.add(`${sessionCode}:${expectedRunCode}`);
    observedRecoveredRunCodesRef.current.delete(sessionCode);
    if (localOwner) detachLocalRunObserver(sessionCode);
    if (activeSessionCodeRef.current === sessionCode) {
      setActiveRunRefreshKey((current) => current + 1);
      setActiveChatReplayRefreshKey((current) => current + 1);
      setActiveChatDataRefreshKey((current) => current + 1);
    }
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
    if (sendingRef.current) return;
    if (!activeSession) return;
    const rawContent = contentOverride ?? composer;
    const goalCommand = contentOverride === undefined
      ? parseGoalCommand(rawContent)
      : { isGoalCommand: false, content: rawContent.trim() };
    const content = goalCommand.content;
    const attachments = attachmentOverride ?? pendingAttachments;
    if (!content && !attachments.length) {
      toast.warning(
        goalCommand.isGoalCommand ? "Goal content is required" : "Message content is required",
        { title: goalCommand.isGoalCommand ? "Goal required" : "Nothing to send" },
      );
      return;
    }
    if (!selectedModel) {
      toast.warning("Select a model before sending", { title: "Model required" });
      return;
    }

    if (goalCommand.isGoalCommand) {
      const goalSet = await setGoalFromComposerCommand(content);
      if (!goalSet) return;
    }

    const sendInput = createSendInput(activeSession, content, attachments);
    if (!sendInput) return;
    await runChatInput(
      activeSession,
      sendInput,
      contentOverride === undefined,
    );
  }

  async function setGoalFromComposerCommand(objective: string) {
    if (!activeSession) return false;
    const sessionCode = activeSession.sessionCode;
    setGoalActionSessionCode(sessionCode);
    try {
      await setChatSessionGoal({
        sessionCode,
        objective,
        maxRuns: 20,
      });
      setShortTermMemoryRefreshKey((current) => current + 1);
      openRightSidebar();
      toast.success(t("chat.shortTermMemory.messages.goalSet"), {
        title: t("chat.shortTermMemory.messages.goalUpdatedTitle"),
      });
      return true;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.shortTermMemory.messages.goalFailed"), {
        title: t("chat.shortTermMemory.messages.goalFailedTitle"),
      });
      return false;
    } finally {
      setGoalActionSessionCode("");
    }
  }

  async function runChatInput(
    targetSession: ChatSession,
    sendInput: SendChatMessageInput,
    shouldClearComposer = false,
    sourceSubmission: ChatQueuedSubmission | null = null,
  ) {
    const restoreSourceSubmission = () => {
      if (
        !sourceSubmission
        || invalidatedQueueSessionCodesRef.current.has(sourceSubmission.sessionCode)
      ) return;
      const remaining = removeQueuedChatSubmission(
        queuedSubmissionsRef.current,
        sourceSubmission.id,
      );
      replaceQueuedSubmissions([sourceSubmission, ...remaining]);
    };
    if (streamOwnershipRegistryRef.current.current || sendingRef.current) {
      restoreSourceSubmission();
      return false;
    }
    const content = sendInput.content.trim();
    if (!content && !(sendInput.attachments?.length)) {
      restoreSourceSubmission();
      return false;
    }
    const pendingSubmissionAttachments = sendInput.attachments ?? [];
    const autoDrainEpoch = autoDrainEpochRef.current;
    const sendingSessionCode = targetSession.sessionCode;
    const abortController = new AbortController();
    const streamOwner = streamOwnershipRegistryRef.current.acquire(
      sendingSessionCode,
      abortController,
    );
    const currentStreamSettlement = () => resolveChatStreamSettlement({
      activeSessionCode: activeSessionCodeRef.current,
      detached: detachedStreamControllersRef.current.has(abortController),
      owner: streamOwner,
      registry: streamOwnershipRegistryRef.current,
      stopped: stopRequestedStreamControllersRef.current.has(abortController),
    });
    const liveRunCodes = new Set<string>();
    activeRunCodeRef.current = "";
    activeRunSupportsSteerRef.current = null;
    setActiveRunCode("");
    setActiveRunSupportsSteer(null);
    const sendStartedAt = Date.now();
    const streamAcceptedAfter = sendStartedAt - liveAguiStartToleranceMs;
    const optimisticUserCode = `LOCAL-${sendStartedAt}-USER`;
    const optimisticAssistantCode = `LOCAL-${sendStartedAt}-ASSISTANT`;
    const optimisticCreatedAt = new Date().toISOString();
    const optimisticUserMessage: ChatMessage = {
      actor: "User",
      channel: targetSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content,
      attachments: pendingSubmissionAttachments,
      createdAt: optimisticCreatedAt,
      id: -sendStartedAt,
      messageCode: optimisticUserCode,
      role: "USER",
      runCode: null,
      sessionCode: sendingSessionCode,
    };
    const optimisticAssistantMessage: ChatMessage = {
      actor: "Assistant",
      channel: targetSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content: "",
      createdAt: optimisticCreatedAt,
      id: -sendStartedAt - 1,
      messageCode: optimisticAssistantCode,
      role: "ASSISTANT",
      runCode: null,
      sessionCode: sendingSessionCode,
    };
    setSendingState(true);
    runningSessionCodeRef.current = sendingSessionCode;
    setRunningSessionCode(sendingSessionCode);
    if (shouldClearComposer) {
      setComposer("");
    }
    if (shouldClearComposer) {
      setPendingAttachments([]);
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
    let completed = false;
    let completedRunCode = "";
    let completedSession: ChatSession | null = null;
    // Once a request crosses the transport boundary, automatic retry is unsafe:
    // the backend may already have persisted its USER message. The sole safe
    // restore path is CHAT_RUN_ALREADY_ACTIVE, which rejects before admission.
    let shouldRestoreSourceSubmission = false;
    try {
      const result = await streamChatMessage(
        targetSession,
        sendInput,
        {
          onAguiEvent: (event) => {
            if (!streamOwnershipRegistryRef.current.owns(streamOwner)) return;
            if (event.threadId && event.threadId !== sendingSessionCode) return;
            const eventMillis = aguiEventMillis(event);
            if (eventMillis !== null && eventMillis < streamAcceptedAfter) return;
            const runCode = aguiEventRunCode(event);
            if (String(event.type).toUpperCase() === "RUN_STARTED" && runCode && runCode !== "stream") {
              activeRunCodeRef.current = runCode;
              setActiveRunCode(runCode);
              const supportsSteer = typeof event.supportsSteer === "boolean"
                ? event.supportsSteer
                : null;
              activeRunSupportsSteerRef.current = supportsSteer;
              setActiveRunSupportsSteer(supportsSteer);
            }
            if (runCode && runCode !== "stream") {
              liveRunCodes.add(runCode);
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
            if (!streamOwnershipRegistryRef.current.owns(streamOwner)) return;
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
      if (!currentStreamSettlement().canCommit) {
        // Some transports settle normally after AbortSignal fires. Reuse the
        // same observer-settlement path as a rejected stream before applying
        // any session, usage, transcript, or queue side effects.
        throw new Error("Chat stream observer closed");
      }
      shouldRestoreSourceSubmission = false;
      setSessionGroups((current) => replaceSession(current, result.session));
      scheduleSessionTitleRefresh(result.session);
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
      const runCodes = Array.from(liveRunCodes);
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
        setVisibleMessages((current) => mergeChatMessagesChronologically(
          current,
          result.messages,
          [optimisticUserCode, optimisticAssistantCode],
        ));
      }
      setShortTermMemoryRefreshKey((current) => current + 1);
      setUnseenSessionCodes((current) => {
        const next = new Set(current);
        if (activeSessionCodeRef.current === result.session.sessionCode) next.delete(result.session.sessionCode);
        else next.add(result.session.sessionCode);
        return next;
      });
      completedSession = result.session;
      completedRunCode = result.run.runCode;
      completed = !stopRequestedStreamControllersRef.current.has(abortController)
        && !detachedStreamControllersRef.current.has(abortController)
        && autoDrainEpochRef.current === autoDrainEpoch;
    } catch (error) {
      const settlement = currentStreamSettlement();
      const explicitlyStopped = stopRequestedStreamControllersRef.current.has(abortController);
      if (!settlement.canCommit) {
        shouldRestoreSourceSubmission = settlement.restoreSourceSubmission;
        if (explicitlyStopped) {
          const terminalEvents = stoppedAguiEvents(sendingSessionCode, liveRunCodes);
          if (terminalEvents.length) {
            setLiveAguiEventsBySessionCode((current) => ({
              ...current,
              [sendingSessionCode]: mergeTranscriptAguiEvents(current[sendingSessionCode] ?? [], terminalEvents),
            }));
          }
        }
        if (activeSessionCodeRef.current === sendingSessionCode) {
          setVisibleMessages((current) => current.filter((item) => (
            item.messageCode !== optimisticAssistantCode || Boolean(item.content.trim())
          )));
        }
      } else if (isChatRunAlreadyActiveError(error)) {
        if (sourceSubmission) {
          restoreSourceSubmission();
        } else {
          enqueueSubmission(targetSession, sendInput, "front");
        }
        setActiveChatReplayRefreshKey((current) => current + 1);
        setActiveRunRefreshKey((current) => current + 1);
        if (activeSessionCodeRef.current === sendingSessionCode) {
          setVisibleMessages((current) => current.filter((item) => (
            item.messageCode !== optimisticUserCode
            && item.messageCode !== optimisticAssistantCode
          )));
        }
        toast.info(
          "Message was kept in the queue while the current response finishes",
          { title: "Active response recovered" },
        );
      } else {
        const rawMessage = error instanceof Error ? error.message : "Chat send failed";
        const toastMessage = chatSendErrorToastMessage(rawMessage);
        if (!sourceSubmission && pendingSubmissionAttachments.length) {
          setPendingAttachments((current) => [...pendingSubmissionAttachments, ...current]);
        }
        if (activeSessionCodeRef.current === sendingSessionCode) {
          setVisibleMessages((current) => current.filter((item) => (
            item.messageCode !== optimisticAssistantCode || Boolean(item.content.trim())
          )));
        }
        toast.error(toastMessage, { title: "Chat send failed" });
      }
    } finally {
      clearLocalStreamOwnership(streamOwner);
    }

    if (shouldRestoreSourceSubmission) restoreSourceSubmission();

    if (completed && completedSession) {
      drainedRecoveredRunKeysRef.current.add(
        `${sendingSessionCode}:${completedRunCode}`,
      );
      if (
        observedRecoveredRunCodesRef.current.get(sendingSessionCode)
        === completedRunCode
      ) {
        observedRecoveredRunCodesRef.current.delete(sendingSessionCode);
      }
      const nextSubmission = takeNextQueuedSubmission(sendingSessionCode);
      if (nextSubmission) {
        void runChatInput(
          completedSession,
          nextSubmission.input,
          false,
          nextSubmission,
        );
      }
    }
    return completed;
  }

  async function handleQueueMessage() {
    if (queueDisabled || !activeSession) return;
    const content = composer.trim();
    if (!content && !pendingAttachments.length) return;
    const sendInput = createSendInput(
      activeSession,
      content,
      pendingAttachments,
    );
    if (!sendInput) return;
    setComposer("");
    setPendingAttachments([]);
    if (sendingRef.current || authoritativeActiveRun || replayActiveRunLoadingHint) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }
    await runChatInput(activeSession, sendInput);
  }

  async function handleSteerMessage() {
    if (steeringRef.current || steerDisabled || !activeSession) return;
    const content = composer.trim();
    if (!content && !pendingAttachments.length) return;
    const sendInput = createSendInput(
      activeSession,
      content,
      pendingAttachments,
    );
    if (!sendInput) return;
    const localRunIsTarget = sendingRef.current
      && runningSessionCodeRef.current === activeSession.sessionCode;
    const expectedRunCode = localRunIsTarget
      ? activeRunCodeRef.current
      : (authoritativeActiveRun?.runCode ?? "");
    const authoritativeRunIsTarget = !localRunIsTarget
      && authoritativeActiveRun?.runCode === expectedRunCode;
    const targetSupportsSteer = localRunIsTarget
      ? activeRunSupportsSteerRef.current === true
      : Boolean(
        authoritativeActiveRun?.supportsSteer
        && authoritativeActiveRun.state.toUpperCase() !== "FINISHING",
      );
    const autoDrainEpoch = autoDrainEpochRef.current;
    setComposer("");
    setPendingAttachments([]);
    if (
      (!localRunIsTarget && !authoritativeRunIsTarget)
      || !expectedRunCode
      || !targetSupportsSteer
    ) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }

    const steerSession = activeSession;
    const abortController = new AbortController();
    steerAbortControllerRef.current = abortController;
    steeringRef.current = true;
    steeringSessionCodeRef.current = steerSession.sessionCode;
    setSteering(true);
    try {
      const result = await steerChatMessage(
        {
          attachments: sendInput.attachments ?? [],
          content: sendInput.content,
          expectedRunCode,
          sessionCode: steerSession.sessionCode,
        },
        abortController.signal,
      );
      if (activeSessionCodeRef.current === result.sessionCode) {
        setVisibleMessages((current) => (
          current.some((message) => message.messageCode === result.message.messageCode)
            ? current
            : [...current, result.message]
        ));
      }
    } catch (error) {
      const sessionInvalidated = invalidatedQueueSessionCodesRef.current.has(
        steerSession.sessionCode,
      );
      const safeQueueFallback = isApiError(error)
        && isSteerQueueFallbackCode(error.code);
      if (!sessionInvalidated && safeQueueFallback) {
        enqueueSubmission(steerSession, sendInput, "front");
        if (
          !sendingRef.current
          && !authoritativeRunIsTarget
          && autoDrainEpochRef.current === autoDrainEpoch
        ) {
          const nextSubmission = takeNextQueuedSubmission(
            steerSession.sessionCode,
          );
          if (nextSubmission) {
            void runChatInput(
              steerSession,
              nextSubmission.input,
              false,
              nextSubmission,
            );
          }
        }
      }
      if (sessionInvalidated) {
        toast.info("Steer message was discarded with its session", { title: "Session closed" });
      } else if (safeQueueFallback) {
        toast.info(
          "The active run rejected steer; the message was queued",
          { title: "Message queued" },
        );
      } else if (isAbortError(error)) {
        toast.info(
          "Steer delivery was not confirmed and was not retried",
          { title: "Steer not retried" },
        );
      } else {
        toast.error(
          error instanceof Error ? error.message : "Chat steer failed",
          { title: "Chat steer failed" },
        );
      }
    } finally {
      if (steerAbortControllerRef.current === abortController) {
        steerAbortControllerRef.current = null;
      }
      steeringRef.current = false;
      steeringSessionCodeRef.current = "";
      setSteering(false);
    }
  }

  async function handleSubmit(mode: ChatSubmitMode = "send") {
    if (mode === "queue") {
      await handleQueueMessage();
      return;
    }
    if (mode === "steer") {
      await handleSteerMessage();
      return;
    }
    await handleSend();
  }

  function handleEditQueuedMessage(id: string, content: string) {
    const submission = queuedSubmissionsRef.current.find((item) => item.id === id);
    replaceQueuedSubmissions(
      removeQueuedChatSubmission(queuedSubmissionsRef.current, id),
    );
    setComposer(content);
    setPendingAttachments(submission?.input.attachments ?? []);
  }

  function handleRemoveQueuedMessage(id: string) {
    replaceQueuedSubmissions(
      removeQueuedChatSubmission(queuedSubmissionsRef.current, id),
    );
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
    if (!activeSession || goalActionSessionCode) return false;
    const sessionCode = activeSession.sessionCode;
    setGoalActionSessionCode(sessionCode);
    try {
      await action();
      setShortTermMemoryRefreshKey((current) => current + 1);
      toast.success(successMessage, {
        title: t("chat.shortTermMemory.messages.goalUpdatedTitle"),
      });
      return true;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.shortTermMemory.messages.goalFailed"), {
        title: t("chat.shortTermMemory.messages.goalFailedTitle"),
      });
      return false;
    } finally {
      setGoalActionSessionCode("");
    }
  }

  async function handleUpdateGoal(input: UpdateChatSessionGoalInput) {
    return runGoalAction(
      () => updateChatSessionGoal(input),
      t("chat.shortTermMemory.messages.goalSaved"),
    );
  }

  async function handleCreateGoal(input: SetChatSessionGoalInput) {
    if (sending) return false;
    if (!selectedModel) {
      toast.warning("Select a model before setting the goal", { title: "Model required" });
      return false;
    }
    const created = await runGoalAction(
      () => setChatSessionGoal(input),
      t("chat.shortTermMemory.messages.goalSet"),
    );
    if (created) {
      openRightSidebar();
      await handleSend(input.objective, []);
    }
    return created;
  }

  async function handlePauseGoal(input: ChatSessionGoalCommand) {
    return runGoalAction(
      () => pauseChatSessionGoal(input),
      t("chat.shortTermMemory.messages.goalPaused"),
    );
  }

  async function handleResumeGoal(input: ChatSessionGoalCommand) {
    if (sending) return false;
    if (!selectedModel) {
      toast.warning("Select a model before resuming the goal", { title: "Model required" });
      return false;
    }
    const resumed = await runGoalAction(
      () => resumeChatSessionGoal(input),
      t("chat.shortTermMemory.messages.goalResumed"),
    );
    if (resumed) {
      await handleSend(t("chat.shortTermMemory.goalResumePrompt"), []);
    }
    return resumed;
  }

  async function handleCancelGoal(input: ChatSessionGoalCommand) {
    return runGoalAction(
      () => cancelChatSessionGoal(input),
      t("chat.shortTermMemory.messages.goalCancelled"),
    );
  }

  async function handleClearGoal(input: ChatSessionGoalCommand) {
    return runGoalAction(
      () => clearChatSessionGoal(input),
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
      invalidatedQueueSessionCodesRef.current.delete(session.sessionCode);
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
      await stopSessionRunBeforeMutation(session.sessionCode);
      const archived = await archiveChatSession(session.sessionCode);
      invalidateQueuedSession(archived.sessionCode);
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
      await stopSessionRunBeforeMutation(session.sessionCode);
      const deleted = await deleteChatSession(session.sessionCode);
      invalidateQueuedSession(deleted.sessionCode);
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

  // Keep automatic width-based collapse separate from the user's manual choice.
  useEffect(() => {
    const mainElement = mainRef.current;
    if (!mainElement) return;
    const resizeObserver = new ResizeObserver(
      throttle((entries: ResizeObserverEntry[]) => {
        for (const entry of entries) {
          if (entry.contentBoxSize) {
            const width = entry.contentBoxSize[0].inlineSize;
            setAutoSidebarCollapsed(width < 600);
          }
        }
      }, 10)
    );

    resizeObserver.observe(mainElement);

    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <div className="relative z-0 flex h-full min-h-0 min-w-0 overflow-hidden overscroll-none bg-white" ref={mainRef}>
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
        onCollapseToggle={() => setUserSidebarCollapsed((value) => !value)}
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
          attachmentDownloader={activeSession
            ? (attachment) => downloadChatSessionAttachment(activeSession, attachment)
              .then((response) => response.blob)
            : undefined}
          autoFocusComposer={shouldAutoFocusChatComposer()}
          composerRef={composerRef}
          composerHeader={transcriptAttachmentHeader}
          createSessionLabel="Create session"
          currentUserDisplayName={currentUserDisplayName}
          debugOptions={transcriptDebugOptions}
          draft={composer}
          emptyDescription={activeWorkspaceCode
            ? "Create a new session or pick one from the left panel to view chat details and start messaging."
            : "Select or create a workspace before loading chat sessions."}
          footerAddon={transcriptFooterAddon}
          formatAnnotationCount={formatAnnotationCount}
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
          onEditQueuedMessage={handleEditQueuedMessage}
          onRemoveQueuedMessage={handleRemoveQueuedMessage}
          onSend={(mode) => {
            void handleSubmit(mode);
          }}
          onStop={() => {
            void handleStopRun();
          }}
          protocolEvents={aguiProtocolEvents}
          queueDisabled={queueDisabled}
          queuedMessages={visibleQueuedSubmissions}
          runningSubmitModes={runningSubmitModes}
          sendDisabled={!currentSessionRunActive && (sending || !activeSession || !selectedModel)}
          sending={currentSessionRunActive}
          session={activeSession}
          showHeader={false}
          showStop={canStopCurrentRun}
          steerDisabled={steerDisabled}
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
