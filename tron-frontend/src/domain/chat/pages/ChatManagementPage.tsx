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
  MessageCircleQuestionMark,
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

import { Badge } from "@/components/ui/badge";
import { InfoTooltip } from "@/components/info-tooltip";
import { MarkdownContent } from "@/components/markdown-content";
import {
  useWorkspaceSpaces,
  useTabPanelConfig,
  useDismissibleLayer,
  useOpenRightSidebar,
  PageView,
  type TargetPageFrameInspectionResult,
  useRightPanelTabs,
} from "@/shared";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useToast } from "@/components/ui/toast";
import { MarkdownLink } from "@/components/markdown-link";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { cn } from "@/shared/lib/utils";
import { CREATE_CHAT_SESSION_SHORTCUT_EVENT } from "@/shared/lib/keyboardShortcuts";
import {
  archiveChatSession,
  createChatSession,
  deleteChatSession,
  getChatActiveRun,
  listChatUserInputRequests,
  stopChatRun,
  steerChatMessage,
  streamChatUserInputAnswer,
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
  useChatWorkRailLabels,
} from "@/domain/chat/hooks";
import { useAuthSession } from "@/domain/auth";
import { useTranslation } from "react-i18next";
import { useTabStore } from "@/store/tabStore";
import { fetchWorkspaceInternalAgents, fetchWorkspaceSystemAgents } from "@/domain/agent/api";
import { listWorkspaceInternalAgentsMock, listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import type { SystemAgentModelRecord, WorkspaceInternalAgentRecord, WorkspaceSystemAgentRecord } from "@/domain/agent/model";
import { fetchWorkspaceSelectOptions } from "@/domain/model/api";
import { getWorkspaceSelectOptions } from "@/domain/model/mock";
import type {
  ModelSelectOption,
  ModelSelection,
  ReasoningEffort,
} from "@/domain/model/model";
import {
  formatReasoningEffort,
  reasoningEffortOptions,
} from "@/domain/model/constants/modelPageOptions";
import { getApiAccessToken, isApiError, requestApi } from "@/shared/api/httpClient";
import { chatApiEndpoints } from "@/domain/chat/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useAguiWorkReplayPreferences } from "@/domain/settings/model/aguiWorkReplayPreferences";
import { TronChatRuntimeWindow } from "@tron/chat-react";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import {
  createMockChatAdapter,
  enqueueChatSubmission,
  isSteerQueueFallbackCode,
  mergeChatMessagesChronologically,
  removeQueuedChatSubmission,
  reasoningLastSentence,
  takeNextQueuedChatSubmission,
  toolCommandLinePreview,
  type ChatQueuedSubmission,
  type ChatAttachmentLabels,
  type ChatSubmitMode,
  type ChatUserInputLabels,
  type ChatRuntimeRunState,
  type ChatSession as RuntimeChatSession,
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
  ChatUserInputAnswer,
  ChatUserInputRequest,
  SetChatSessionGoalInput,
  UpdateChatSessionGoalInput,
} from "@/domain/chat/model";
import { EditableTabs, type EditableTab } from "@/components/editable-tabs";
import { useInspection } from "../hooks/useInspection";
import {
  downloadChatSessionAttachment,
  uploadChatSessionAttachment,
} from "../api/chatAttachmentApi";
import DraggleLine from "@/components/draggle-line";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
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

function sessionModelSelectionKey(sessionCode: string, agentCode: string) {
  return `${sessionCode}::${agentCode.trim().toLowerCase() || "main-agent"}`;
}

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
      selections[sessionCode.includes("::") ? sessionCode : `${sessionCode}::main-agent`] = optionCode;
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

const chatSessionSourceTabs: { id: ChatSessionSourceTab; icon: LucideIcon; labelKey: string }[] = [
  { id: "chat", icon: MessageSquareText, labelKey: "chat.management.sourceTabs.chat" },
  { id: "automation", icon: AlarmClock, labelKey: "chat.management.sourceTabs.automation" },
  { id: "task", icon: ListTodo, labelKey: "chat.management.sourceTabs.task" },
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
  const { t } = useTranslation("pages");
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
          aria-label={t("chat.management.composer.removeAttachment", { label })}
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
        aria-label={t("chat.management.composer.removeAttachment", { label })}
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
    if ((left.pendingUserInputCount ?? 0) !== (right.pendingUserInputCount ?? 0)) {
      return (right.pendingUserInputCount ?? 0) - (left.pendingUserInputCount ?? 0);
    }
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

type TranscriptAguiEvent = ChatAguiEvent | ChatAguiStreamEvent;

function aguiEventMetadata(event: TranscriptAguiEvent) {
  if (!("metadata" in event) || typeof event.metadata !== "object" || event.metadata === null) {
    return null;
  }
  return event.metadata as Record<string, string>;
}

function latestEventValue(events: TranscriptAguiEvent[], key: string) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const metadata = aguiEventMetadata(events[index]);
    if (metadata?.[key]) return metadata[key];
  }
  return null;
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

export function normalizeContextBudgetBreakdown(parsed: Partial<ChatContextBudgetBreakdown> | null | undefined): ChatContextBudgetBreakdown | null {
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
    toolSchemaBudgetTokens: tokenField(parsed.toolSchemaBudgetTokens),
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

function latestUsageEstimateFromEvents(events: TranscriptAguiEvent[]) {
  return parsedUsageEstimate(latestEventValue(events, "usage_estimate_json"))
    ?? parsedUsageEstimate(latestEventValue(events, "usageEstimate"));
}

function latestUsageAuditFromEvents(events: TranscriptAguiEvent[]) {
  return parsedUsageAudit(latestEventValue(events, "usage_audit_json"));
}

function sessionProviderUsageFromEvents(events: TranscriptAguiEvent[]): ChatTokenUsage | null {
  const seen = new Set<string>();
  const totals = { inputTokens: 0, outputTokens: 0, totalTokens: 0 };
  let hasUsage = false;
  let cachedInputTokens = 0;
  let hasCachedInput = false;
  let reasoningTokens = 0;
  let hasReasoning = false;

  events.forEach((event) => {
    const metadata = aguiEventMetadata(event);
    const audit = parsedUsageAudit(metadata?.usage_audit_json);
    const usage = audit?.providerUsage;
    if (!usage) return;

    const eventId = "id" in event && typeof event.id === "string" ? event.id : "";
    const key = eventId || [
      aguiEventRunCode(event),
      aguiEventTime(event),
      audit.responseId ?? "",
      usage.inputTokens,
      usage.outputTokens,
      usage.totalTokens,
    ].join("\u0000");
    if (seen.has(key)) return;
    seen.add(key);

    hasUsage = true;
    totals.inputTokens += usage.inputTokens;
    totals.outputTokens += usage.outputTokens;
    totals.totalTokens += usage.totalTokens;
    if (usage.cachedInputTokens != null) {
      hasCachedInput = true;
      cachedInputTokens += usage.cachedInputTokens;
    }
    if (usage.reasoningTokens != null) {
      hasReasoning = true;
      reasoningTokens += usage.reasoningTokens;
    }
  });

  if (!hasUsage) return null;
  return {
    ...totals,
    cachedInputTokens: hasCachedInput ? cachedInputTokens : null,
    reasoningTokens: hasReasoning ? reasoningTokens : null,
  };
}

function shortTermMemoryEventPayload(event: TranscriptAguiEvent): Record<string, unknown> | null {
  if ("value" in event && typeof event.value === "object" && event.value !== null) {
    return event.value as Record<string, unknown>;
  }
  const encoded = aguiEventMetadata(event)?.context_event_json;
  if (!encoded) return null;
  try {
    const parsed = JSON.parse(encoded) as unknown;
    return typeof parsed === "object" && parsed !== null
      ? parsed as Record<string, unknown>
      : null;
  } catch {
    return null;
  }
}

export function latestContextBudgetBreakdownFromEvents(events: TranscriptAguiEvent[]) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const payload = shortTermMemoryEventPayload(events[index]);
    const breakdown = payload?.budgetBreakdown;
    if (typeof breakdown === "object" && breakdown !== null) {
      const normalized = normalizeContextBudgetBreakdown(
        breakdown as Partial<ChatContextBudgetBreakdown>,
      );
      if (normalized) return normalized;
    }
    const normalized = parsedContextBudgetBreakdown(
      aguiEventMetadata(events[index])?.budgetBreakdown,
    );
    if (normalized) return normalized;
  }
  return null;
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
  model: ChatModelOption | null | undefined,
  selectedModel: ChatModelOption | null | undefined,
  events: TranscriptAguiEvent[],
) {
  const metadataWindow = tokenFieldFromMetadata(
    contextValue(latestEventValue(events, "contextWindowTokens"))
    ?? contextValue(latestEventValue(events, "context_window_tokens"))
    ?? contextValue(latestEventValue(events, "contextWindowLimit"))
    ?? contextValue(latestEventValue(events, "context_window_limit")),
  );
  return model?.contextWindowTokens ?? selectedModel?.contextWindowTokens ?? metadataWindow ?? null;
}

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

export function shouldPollChatRecoveryData({
  activeRunConfirmedIdle,
  hasActiveRun,
  hasOpenReplayRun,
  hasPendingUserInput,
  hasQueuedSubmission,
}: {
  activeRunConfirmedIdle: boolean;
  hasActiveRun: boolean;
  hasOpenReplayRun: boolean;
  hasPendingUserInput: boolean;
  hasQueuedSubmission: boolean;
}) {
  return hasActiveRun
    || (hasOpenReplayRun && !activeRunConfirmedIdle)
    || hasQueuedSubmission
    || hasPendingUserInput;
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

function aguiEventMillis(event: TranscriptAguiEvent) {
  if ("timestamp" in event && typeof event.timestamp === "number") return event.timestamp;
  const parsed = Date.parse(aguiEventTime(event));
  return Number.isNaN(parsed) ? null : parsed;
}

export type ShortTermMemoryRuntimeNoticeKind =
  | "compaction_requested"
  | "compaction_started"
  | "compaction_completed"
  | "compaction_failed"
  | "tool_schema_over_budget";

export interface ShortTermMemoryRuntimeNotice {
  kind: ShortTermMemoryRuntimeNoticeKind;
  timestamp: number;
  estimatedTokens?: number | null;
  limitTokens?: number | null;
  toolCount?: number | null;
}

function shortTermMemoryEventName(event: TranscriptAguiEvent) {
  if ("name" in event && typeof event.name === "string") return event.name;
  if ("eventName" in event && typeof event.eventName === "string") return event.eventName;
  return "";
}

function numberFromUnknown(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function compactionNoticeKind(event: TranscriptAguiEvent): ShortTermMemoryRuntimeNoticeKind | null {
  switch (shortTermMemoryEventName(event)) {
    case "tron.short_term_memory.context.compaction_requested":
    case "agent.context.compaction_requested":
      return "compaction_requested";
    case "tron.short_term_memory.compaction.started":
    case "agent.compaction.started":
      return "compaction_started";
    case "tron.short_term_memory.compaction.completed":
    case "agent.compaction.completed":
      return "compaction_completed";
    case "tron.short_term_memory.compaction.failed":
    case "agent.compaction.failed":
      return "compaction_failed";
    default:
      return null;
  }
}

function isRunFinishedEvent(event: TranscriptAguiEvent) {
  const type = aguiEventType(event).toUpperCase();
  return type === "RUN_FINISHED" || type === "AGENT.RUN.COMPLETED";
}

function isRunErrorEvent(event: TranscriptAguiEvent) {
  const type = aguiEventType(event).toUpperCase();
  return type === "RUN_ERROR" || type === "AGENT.RUN.FAILED";
}

function toolSchemaBudgetFields(message: string) {
  const matched = message.match(
    /estimatedTokens=(\d+),\s*limitTokens=(\d+),\s*toolCount=(\d+)/i,
  );
  return {
    estimatedTokens: matched ? Number.parseInt(matched[1], 10) : null,
    limitTokens: matched ? Number.parseInt(matched[2], 10) : null,
    toolCount: matched ? Number.parseInt(matched[3], 10) : null,
  };
}

export function latestShortTermMemoryRuntimeNotice(
  events: TranscriptAguiEvent[],
): ShortTermMemoryRuntimeNotice | null {
  let compactionNotice: ShortTermMemoryRuntimeNotice | null = null;
  let toolSchemaNotice: ShortTermMemoryRuntimeNotice | null = null;
  let latestSuccessfulRunAt = Number.NEGATIVE_INFINITY;

  for (let index = 0; index < events.length; index += 1) {
    const event = events[index];
    const timestamp = aguiEventMillis(event) ?? index;
    const kind = compactionNoticeKind(event);
    if (kind) {
      const payload = shortTermMemoryEventPayload(event);
      compactionNotice = {
        kind,
        timestamp,
        estimatedTokens: numberFromUnknown(payload?.compactionInputTokenEstimate)
          ?? numberFromUnknown(payload?.estimatedTokens),
        limitTokens: numberFromUnknown(payload?.maxInputTokens),
      };
    }

    if (isRunFinishedEvent(event)) {
      latestSuccessfulRunAt = Math.max(latestSuccessfulRunAt, timestamp);
      continue;
    }
    if (!isRunErrorEvent(event)) continue;
    const metadata = aguiEventMetadata(event);
    const code = (
      "code" in event && typeof event.code === "string"
        ? event.code
        : metadata?.failureCode ?? ""
    ).toLowerCase();
    const message = aguiEventSummary(event);
    if (code !== "tool_schema_over_budget" && !message.includes("TOOL_SCHEMA_OVER_BUDGET")) {
      continue;
    }
    const fields = toolSchemaBudgetFields(message);
    toolSchemaNotice = {
      kind: "tool_schema_over_budget",
      timestamp,
      ...fields,
    };
  }

  if (toolSchemaNotice && toolSchemaNotice.timestamp < latestSuccessfulRunAt) {
    toolSchemaNotice = null;
  }
  if (!toolSchemaNotice) return compactionNotice;
  if (!compactionNotice) return toolSchemaNotice;
  return toolSchemaNotice.timestamp >= compactionNotice.timestamp
    ? toolSchemaNotice
    : compactionNotice;
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
  return explicitAguiEventToolName(event) ?? "tool";
}

function explicitAguiEventToolName(event: TranscriptAguiEvent) {
  if ("toolCallName" in event && typeof event.toolCallName === "string" && event.toolCallName.trim()) {
    return event.toolCallName;
  }
  return null;
}

function updateToolCardName(card: ToolWorkCard, event: TranscriptAguiEvent) {
  const toolName = explicitAguiEventToolName(event);
  if (toolName) card.toolName = toolName;
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

function toolActionLabel(toolName: string, args: unknown, t: (key: string) => string) {
  const normalizedTool = toolName.toLowerCase();
  const target = valueAtKnownKey(args).toLowerCase();
  if (normalizedTool.includes("edit") || normalizedTool.includes("write") || normalizedTool.includes("patch")) return t("chat.management.tool.actionEdit");
  if (target.includes("/") || target.includes(".")) {
    if (!normalizedTool.includes("exec") && !normalizedTool.includes("run") && !normalizedTool.includes("shell")) return t("chat.management.tool.actionEdit");
  }
  return t("chat.management.tool.actionRun");
}

function toolSummaryLabel(toolName: string, args: unknown, t: (key: string) => string) {
  const displayName = toolDisplayName(toolName, args);
  return `${toolActionLabel(displayName, args, t)} ${displayName}`;
}

function toolDisplayName(toolName: string, args: unknown) {
  const normalizedName = toolName.trim();
  const domainCode = domainCodeFromToolPayload(normalizedName) || domainCodeFromToolPayload(args);
  if (normalizedName === "discovery_expand_tool_domain" || domainCode) {
    return humanizeToolName(domainCode || normalizedName);
  }
  return humanizeToolName(normalizedName);
}

function domainCodeFromToolPayload(value: unknown) {
  const payload = parseToolPayload(value);
  return payload && typeof payload.domainCode === "string" ? payload.domainCode.trim() : "";
}

function parseToolPayload(value: unknown): Record<string, unknown> | null {
  if (value && typeof value === "object" && !Array.isArray(value)) return value as Record<string, unknown>;
  if (typeof value !== "string") return null;
  try {
    const parsed = JSON.parse(value) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : null;
  } catch {
    return null;
  }
}

function humanizeToolName(value: string) {
  return value
    .trim()
    .split(/[._-]+/)
    .filter(Boolean)
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(" ") || "tool";
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
  summary: string;
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
    summary: "",
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
      card.summary = appendReasoningDelta(card.summary, delta);
      return;
    }
    if (type === "REASONING_TEXT_CONTENT") {
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
      updateToolCardName(card, event);
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
      updateToolCardName(card, event);
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
      .filter((value) => value.content.trim() || value.summary.trim() || !value.ended)
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

function availableChatModels(models: ModelSelectOption[]) {
  // Keep unavailable fixed models in this list. A persisted session selection
  // must remain visible (and disabled) instead of silently falling back.
  return models.filter((model) => model.modelType === "TEXT"
    && !(model.modelSelection?.kind === "ProviderRoute" && model.modelSelection.routeCode === "ChatAuto"));
}

interface ChatModelOption {
  accessPlatform: string | null;
  cacheHitCoefficient: number | null;
  contextWindowTokens: number | null;
  defaultReasoningEffort: ReasoningEffort | null;
  displayName: string;
  enabled: boolean;
  inputCoefficient: number | null;
  isDefault: boolean;
  modelCode: string | null;
  modelName: string;
  modelSelection: ModelSelection | null;
  optionCode: string;
  providerCode: string | null;
  remoteStatus: string | null;
  qualityType: "High" | "Middle" | "Low" | null;
  source: "internal-agent" | "system-agent";
  supportedReasoningEfforts: ReasoningEffort[];
  upstreamVendorName: string | null;
  outputCoefficient: number | null;
  vendor: string | null;
}

interface QueryStatus {
  loading?: boolean;
  error?: Error | null;
  retry?: () => void;
}

interface ChatRightPanelSnapshot {
  compactionBusy: boolean;
  contextEvents: TranscriptAguiEvent[];
  goal?: ChatSessionGoalView | null;
  goalBusy: boolean;
  goalStatus: QueryStatus;
  latestCompaction?: ChatSessionCompaction | null;
  latestCompactionStatus: QueryStatus;
  latestRequestEstimate?: ChatTokenEstimateSnapshot | null;
  latestUsageAudit?: ChatUsageAuditSnapshot | null;
  messages: ChatMessage[];
  model?: ChatModelOption | null;
  selectedModel?: ChatModelOption | null;
  session: ChatSession;
  todos?: ChatSessionTodo[];
  todosStatus: QueryStatus;
  workspaceGoalRunning: boolean;
}

interface ChatAgentOption {
  agentCode: string;
  description: string;
  models: SystemAgentModelRecord[];
  name: string;
  kind: "internal" | "system";
}

const mainChatAgentOption: ChatAgentOption = {
  agentCode: "main-agent",
  description: "Platform main in-process agent runtime.",
  models: [],
  name: "Main Agent",
  kind: "internal",
};

function availableChatAgents(internalRecords: WorkspaceInternalAgentRecord[], systemRecords: WorkspaceSystemAgentRecord[]) {
  const internalAgents: ChatAgentOption[] = internalRecords.map((record) => ({
    agentCode: record.agentCode,
    description: record.description,
    models: [] as SystemAgentModelRecord[],
    name: record.name,
    kind: "internal" as const,
  }));
  if (!internalAgents.some((agent) => agent.agentCode === mainChatAgentOption.agentCode)) {
    internalAgents.unshift(mainChatAgentOption);
  }
  const systemAgents = systemRecords
    .filter((record) => record.available && record.enabled)
    .map((record) => ({
      agentCode: record.agentCode,
      description: record.description,
      models: record.models,
      name: record.name,
      kind: "system" as const,
    }));
  return [...internalAgents, ...systemAgents];
}

function isMainChatAgent(agentCode: string) {
  return agentCode.trim().toLowerCase() === mainChatAgentOption.agentCode;
}

function normalizeModelSelection(model: ModelSelectOption): ModelSelection {
  return model.modelSelection ?? {
    kind: "Fixed",
    providerCode: null,
    modelCode: model.modelCode,
    qualityType: null,
    routeCode: null,
  };
}

function modelsForAgent(models: ModelSelectOption[], selectedAgent: ChatAgentOption | null): ChatModelOption[] {
  if (!selectedAgent || selectedAgent.kind === "internal") {
    const fixedModels = models.map((model) => ({
      accessPlatform: model.accessPlatform ?? null,
      cacheHitCoefficient: model.cacheHitCoefficient ?? null,
      contextWindowTokens: model.contextWindowTokens ?? null,
      defaultReasoningEffort: model.defaultReasoningEffort ?? null,
      displayName: model.displayName,
      enabled: model.enabled,
      inputCoefficient: model.inputCoefficient ?? null,
      isDefault: Boolean(model.isDefault),
      modelCode: model.modelCode,
      modelName: model.modelName,
      modelSelection: normalizeModelSelection(model),
      optionCode: model.optionCode ?? `main:${model.modelCode}`,
      providerCode: model.providerCode ?? null,
      remoteStatus: model.remoteStatus ?? null,
      qualityType: model.qualityType ?? model.modelSelection?.qualityType ?? null,
      source: "internal-agent" as const,
      supportedReasoningEfforts: model.supportedReasoningEfforts ?? [],
      upstreamVendorName: model.upstreamVendorName ?? null,
      outputCoefficient: model.outputCoefficient ?? null,
      vendor: model.vendor ?? null,
    }));
    return [autoChatModelOption(selectedAgent?.agentCode ?? mainChatAgentOption.agentCode), ...fixedModels];
  }

  return selectedAgent.models.map((model) => ({
    accessPlatform: null,
    cacheHitCoefficient: null,
    contextWindowTokens: model.contextWindowTokens ?? null,
    defaultReasoningEffort: null,
    displayName: model.displayName || model.modelId,
    enabled: true,
    inputCoefficient: null,
    isDefault: false,
    modelCode: null,
    modelName: model.modelId,
    modelSelection: null,
    optionCode: `system:${selectedAgent.agentCode}:${model.modelId}`,
    providerCode: null,
    remoteStatus: null,
    qualityType: null,
    source: "system-agent" as const,
    supportedReasoningEfforts: [],
    upstreamVendorName: null,
    outputCoefficient: null,
    vendor: null,
  }));
}

function autoChatModelOption(agentCode: string): ChatModelOption {
  return {
    accessPlatform: null,
    cacheHitCoefficient: null,
    contextWindowTokens: null,
    defaultReasoningEffort: null,
    displayName: "Auto",
    enabled: true,
    inputCoefficient: null,
    isDefault: true,
    modelCode: null,
    modelName: "",
    modelSelection: {
      kind: "Auto",
      providerCode: null,
      modelCode: null,
      qualityType: null,
      routeCode: null,
    },
    optionCode: `auto:${agentCode}`,
    providerCode: null,
    remoteStatus: null,
    qualityType: null,
    source: "internal-agent",
    supportedReasoningEfforts: [],
    upstreamVendorName: null,
    outputCoefficient: null,
    vendor: null,
  };
}

export function defaultChatModelOption<
  TOption extends {
    enabled: boolean;
    isDefault: boolean;
    modelSelection: ModelSelection | null;
    source: ChatModelOption["source"];
  },
>(options: TOption[]) {
  if (!options.length) return null;
  if (options[0].source === "system-agent") return options[0];
  const enabled = options.filter((option) => option.enabled);
  return enabled.find((option) => option.modelSelection?.kind === "Auto")
    ?? enabled.find((option) => option.modelSelection?.kind === "ProviderRoute"
    && option.modelSelection.routeCode === "ChatAuto")
    ?? enabled.find((option) => option.isDefault)
    ?? enabled[0]
    ?? options[0];
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
  return fuzzyTextMatch(query, model.displayName)
    || fuzzyTextMatch(query, model.modelName)
    || fuzzyTextMatch(query, model.accessPlatform ?? "")
    || fuzzyTextMatch(query, model.upstreamVendorName ?? model.vendor ?? "");
}

function isPersonAiAutoModel(model: ChatModelOption) {
  return model.modelSelection?.kind === "Auto"
    || (model.modelSelection?.kind === "ProviderRoute" && model.modelSelection.routeCode === "ChatAuto");
}

function formatModelMultiplier(value: number | null) {
  if (value === null || !Number.isFinite(value)) return null;
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 6 }).format(value);
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
  const { t } = useTranslation("pages");

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
        aria-label={t("chat.management.sidebar.openActionsAria", { title: session.title })}
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
          aria-label={t("chat.management.sidebar.actionsAria", { title: session.title })}
        >
          <button className="flex w-full rounded-md px-2.5 py-1.5 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={() => select(onRename)}>
            {t("chat.management.actions.rename")}
          </button>
          <button className="flex w-full rounded-md px-2.5 py-1.5 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={() => select(onArchive)}>
            {t("chat.management.actions.archive")}
          </button>
          <button className="flex w-full rounded-md px-2.5 py-1.5 text-left text-[var(--color-error)] hover:bg-[var(--color-error-container)]" type="button" role="menuitem" onClick={() => select(onDelete)}>
            {t("chat.management.actions.delete")}
          </button>
        </div>
      ) : null}
    </div>
  );
}

function SessionRunIndicator({
  needsUserInput,
  running,
  unseen,
}: {
  needsUserInput: boolean;
  running: boolean;
  unseen: boolean;
}) {
  const { t } = useTranslation("pages");
  if (running) {
    return (
      <span className="inline-flex size-5 items-center justify-center" aria-label={t("chat.management.runIndicator.running")} title={t("chat.management.runIndicator.running")}>
        <LoaderCircle className="animate-spin text-[var(--color-focus-ring)]" size={15} />
      </span>
    );
  }

  if (needsUserInput) {
    return (
      <span className="inline-flex size-5 items-center justify-center" aria-label={t("chat.management.runIndicator.waitingAnswer")} title={t("chat.management.runIndicator.waitingAnswer")}>
        <MessageCircleQuestionMark className="text-[var(--color-focus-ring)]" size={15} />
      </span>
    );
  }

  if (unseen) {
    return (
      <span className="inline-flex size-5 items-center justify-center" aria-label={t("chat.management.runIndicator.unread")} title={t("chat.management.runIndicator.unread")}>
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
  detailCode?: "systemPrompt" | "toolSchema" | "familyContext";
  limitTokens: number | null;
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

export function buildContextMemoryBlocks({
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
  const checkpointStat = latestCompaction || checkpointCode
    ? translate("chat.shortTermMemory.contextPanel.checkpointStat")
    : checkpointTokens > 0
      ? translate("chat.shortTermMemory.contextPanel.loaded")
      : translate("chat.shortTermMemory.contextPanel.none");
  const sharedInputBudget = breakdown?.effectiveInputBudget ?? requestEstimate?.effectiveInputBudget ?? null;
  const toolSchemaTokens = breakdown?.toolSchemaTokens ?? requestEstimate?.toolSchemaTokenEstimate ?? 0;
  const toolSchemaLimit = breakdown?.toolSchemaBudgetTokens ?? sharedInputBudget ?? breakdown?.contextWindowLimit ?? requestEstimate?.contextWindowLimit ?? null;
  return [
    {
      code: "01",
      color: "seed",
      detailCode: "systemPrompt",
      limitTokens: breakdown?.promptPrefixBudgetTokens ?? null,
      name: translate("chat.shortTermMemory.contextPanel.systemPromptName"),
      stat: translate("chat.shortTermMemory.contextPanel.messageCount", {
        count: breakdown?.promptPrefixMessageCount ?? 0,
      }),
      tokens: breakdown?.promptPrefixTokens ?? 0,
    },
    {
      code: "02",
      color: "tool",
      detailCode: "toolSchema",
      limitTokens: toolSchemaLimit,
      name: translate("chat.shortTermMemory.contextPanel.toolSchemaName"),
      stat: "",
      tokens: toolSchemaTokens,
    },
    {
      code: "03",
      color: "checkpoint",
      limitTokens: breakdown?.checkpointBudgetTokens ?? null,
      name: translate("chat.shortTermMemory.contextPanel.checkpointName"),
      stat: hasCheckpointInRequest ? checkpointStat : translate("chat.shortTermMemory.contextPanel.none"),
      tokens: checkpointTokens,
    },
    {
      code: "04",
      color: "state",
      limitTokens: breakdown?.workingStateBudgetTokens ?? null,
      name: translate("chat.shortTermMemory.contextPanel.workingStateName"),
      stat: translate("chat.shortTermMemory.contextPanel.workingStateStat", {
        goals: activeGoalCount,
        todos: activeTodoCount,
      }),
      tokens: breakdown?.workingStateTokens ?? 0,
    },
    {
      code: "05",
      color: "family",
      detailCode: "familyContext",
      limitTokens: breakdown?.sessionFamilyContextBudgetTokens ?? null,
      name: translate("chat.shortTermMemory.contextPanel.familyContextName"),
      stat: breakdown?.sessionFamilyContextTokens
        ? translate("chat.shortTermMemory.contextPanel.loaded")
        : translate("chat.shortTermMemory.contextPanel.none"),
      tokens: breakdown?.sessionFamilyContextTokens ?? 0,
    },
    {
      code: "06",
      color: "history",
      limitTokens: breakdown?.currentSessionHistoryBudgetTokens ?? null,
      name: translate("chat.shortTermMemory.contextPanel.sessionHistoryName"),
      stat: translate("chat.shortTermMemory.contextPanel.messageCount", { count: historyMessageCount }),
      tokens: breakdown?.recentTranscriptTokens ?? requestEstimate?.messageTokenEstimate ?? 0,
    },
  ];
}

function ContextOverviewPanel({
  breakdown,
  compactionBusy,
  fallbackWindowLimit,
  modelLabel,
  onCreateCompaction,
  requestEstimate,
  session,
}: {
  breakdown: ChatContextBudgetBreakdown | null;
  compactionBusy?: boolean;
  fallbackWindowLimit?: number | null;
  modelLabel: string;
  onCreateCompaction?: () => void;
  requestEstimate: ChatTokenEstimateSnapshot | null;
  session?: ChatSession;
}) {
  const { t } = useTranslation("pages");
  const usedTokens = breakdown?.estimatedInputTokens ?? requestEstimate?.estimatedInputTokens ?? (fallbackWindowLimit ? 0 : null);
  const inputBudget = breakdown?.effectiveInputBudget ?? requestEstimate?.effectiveInputBudget ?? fallbackWindowLimit ?? null;
  const windowLimit = breakdown?.contextWindowLimit ?? requestEstimate?.contextWindowLimit ?? fallbackWindowLimit ?? null;
  const percent = contextPercent(usedTokens, inputBudget ?? windowLimit);
  const statusLabel = contextStatusLabel(requestEstimate, breakdown, (key) => t(key));
  const statusTone = contextStatusTone(requestEstimate, breakdown);

  return (
    <div className="min-w-0 space-y-3">
      <div className="flex min-w-0 items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-1">
            <h3 className="text-xs font-semibold uppercase text-[var(--color-on-surface)]">
              {t("chat.shortTermMemory.contextPanel.shortTermMemory")}
            </h3>
            <InfoTooltip ariaLabel={t("chat.shortTermMemory.contextPanel.budgetFormulaAria")} align="start">
              <div className="max-w-[18rem] space-y-1 whitespace-normal leading-5">
                <p className="font-medium">{t("chat.shortTermMemory.contextPanel.budgetFormulaTitle")}</p>
                <p>{t("chat.shortTermMemory.contextPanel.effectiveInputFormula")}</p>
                <p className="whitespace-pre-line">
                  {t("chat.shortTermMemory.contextPanel.historyFormula", {
                    systemPrompt: t("chat.shortTermMemory.contextPanel.systemPromptName"),
                    checkpoint: t("chat.shortTermMemory.contextPanel.checkpointName"),
                    familyContext: t("chat.shortTermMemory.contextPanel.familyContextName"),
                    workingState: t("chat.shortTermMemory.contextPanel.workingStateName"),
                    toolSchema: t("chat.shortTermMemory.contextPanel.toolSchemaName"),
                  })}
                </p>
                <p className="text-[var(--color-on-surface-variant)]">{t("chat.shortTermMemory.contextPanel.budgetFormulaNote")}</p>
              </div>
            </InfoTooltip>
          </div>
          <p className="truncate text-[11px] text-[var(--color-on-surface-variant)]">
            {requestEstimate?.modelName || requestEstimate?.modelCode || modelLabel}
            {windowLimit ? ` · ${formatCompactTokenCount(windowLimit)} ${t("chat.shortTermMemory.contextPanel.windowSuffix")}` : ""}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <Badge className="h-5 rounded px-1.5 text-[11px]" variant={statusTone}>
            {statusLabel}
          </Badge>
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
      </div>
      <div className="flex min-w-0 items-baseline justify-between gap-3">
        <div className="min-w-0 truncate text-2xl font-bold text-[var(--color-on-surface)]">
          {formatCompactTokenCount(usedTokens)}
          <span className="px-1">/</span>
          {formatCompactTokenCount(inputBudget)}
        </div>
        <div className="shrink-0 text-sm font-medium text-[var(--color-on-surface-variant)]">
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
    </div>
  );
}

function ContextRuntimeNoticePanel({ notice }: { notice: ShortTermMemoryRuntimeNotice }) {
  const { t } = useTranslation("pages");
  const pending = notice.kind === "compaction_requested" || notice.kind === "compaction_started";
  const success = notice.kind === "compaction_completed";
  const title = t(`chat.shortTermMemory.contextPanel.runtimeNotices.${notice.kind}.title`);
  const detail = notice.kind === "tool_schema_over_budget"
    && notice.estimatedTokens != null
    && notice.limitTokens != null
    ? t("chat.shortTermMemory.contextPanel.runtimeNotices.tool_schema_over_budget.detailWithBudget", {
      actual: formatCompactTokenCount(notice.estimatedTokens),
      limit: formatCompactTokenCount(notice.limitTokens),
      count: notice.toolCount ?? "-",
    })
    : t(`chat.shortTermMemory.contextPanel.runtimeNotices.${notice.kind}.detail`);

  return (
    <section
      aria-live="polite"
      className={cn(
        "flex items-start gap-2.5 rounded-md border px-3 py-2.5 text-xs",
        pending && "border-[var(--color-outline-variant)] bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
        success && "border-[var(--color-success)]/30 bg-[var(--color-success-container)] text-[var(--color-on-success-container)]",
        !pending && !success && "border-[var(--color-error)]/30 bg-[var(--color-error-container)] text-[var(--color-on-error-container)]",
      )}
      data-notice-kind={notice.kind}
      data-testid="context-runtime-notice"
      role={!pending && !success ? "alert" : "status"}
    >
      {pending ? (
        <LoaderCircle className="mt-0.5 shrink-0 animate-spin" size={14} />
      ) : success ? (
        <CheckCircle2 className="mt-0.5 shrink-0" size={14} />
      ) : (
        <CircleAlert className="mt-0.5 shrink-0" size={14} />
      )}
      <div className="min-w-0 space-y-0.5">
        <div className="font-medium">{title}</div>
        <div className="break-words opacity-80">{detail}</div>
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
  sessionCode,
  blocks,
}: {
  sessionCode: string;
  blocks: ContextMemoryBlockItem[];
}) {
  return (
    <div>
      <div>
        {blocks.map((block) => (
          <ContextMemoryBlockRow block={block} key={block.code} sessionCode={sessionCode} />
        ))}
      </div>
    </div>
  );
}

type ContextPromptPrefixMessage = {
  role: string;
  content: string;
  catalogKey?: string;
  promptCode?: string;
};

function isContextDetailRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isContextPromptPrefixMessage(value: unknown): value is ContextPromptPrefixMessage {
  return isContextDetailRecord(value)
    && typeof value.role === "string"
    && typeof value.content === "string"
    && (value.catalogKey === undefined || typeof value.catalogKey === "string")
    && (value.promptCode === undefined || typeof value.promptCode === "string");
}

function contextPromptPrefixMessages(detail: unknown): ContextPromptPrefixMessage[] | null {
  if (!isContextDetailRecord(detail)) return null;
  const candidates = [detail.promptPrefix, detail];
  for (const candidate of candidates) {
    if (!isContextDetailRecord(candidate)) continue;
    const messages = candidate.messages;
    if (Array.isArray(messages) && messages.every(isContextPromptPrefixMessage)) {
      return messages;
    }
  }
  return null;
}

type ContextPromptContentPart =
  | { kind: "text"; value: string }
  | { kind: "json"; value: unknown };

function embeddedJsonEnd(value: string, start: number) {
  const opening = value[start];
  if (opening !== "{" && opening !== "[") return null;
  const expectedClosing = opening === "{" ? "}" : "]";
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < value.length; index += 1) {
    const character = value[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (character === "\\") {
        escaped = true;
      } else if (character === "\"") {
        inString = false;
      }
      continue;
    }
    if (character === "\"") {
      inString = true;
    } else if (character === opening) {
      depth += 1;
    } else if (character === expectedClosing) {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
  }
  return null;
}

function contextPromptContentParts(content: string): ContextPromptContentPart[] {
  const trimmed = content.trim();
  if (!trimmed) return [];
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (typeof parsed === "string") return [{ kind: "text", value: parsed }];
    return [{ kind: "json", value: parsed }];
  } catch {
    // System prompt templates often embed a JSON runtime value in prose.
  }
  for (let start = 0; start < trimmed.length; start += 1) {
    const end = embeddedJsonEnd(trimmed, start);
    if (end == null) continue;
    try {
      const json = JSON.parse(trimmed.slice(start, end)) as unknown;
      return [
        ...(trimmed.slice(0, start).trim() ? [{ kind: "text" as const, value: trimmed.slice(0, start).trim() }] : []),
        { kind: "json", value: json },
        ...(trimmed.slice(end).trim() ? [{ kind: "text" as const, value: trimmed.slice(end).trim() }] : []),
      ];
    } catch {
      // Continue scanning in case this brace belongs to ordinary prompt text.
    }
  }
  return [{ kind: "text", value: content }];
}

function ContextPromptMessageContent({ content }: { content: string }) {
  const { t } = useTranslation("pages");
  const parts = contextPromptContentParts(content);
  return (
    <div className="space-y-3 text-xs leading-6 text-[var(--color-on-surface)]">
      {parts.map((part, index) => part.kind === "text" ? (
        <pre className="whitespace-pre-wrap break-words font-sans" key={`text-${index}`}>
          {part.value}
        </pre>
      ) : (
        <details className="rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]" key={`json-${index}`}>
          <summary className="cursor-pointer px-3 py-2 text-xs font-medium text-[var(--color-on-surface-variant)] marker:text-[var(--color-on-surface-variant)]">
            {t("chat.shortTermMemory.contextPanel.detailPromptStructuredContent")}
          </summary>
          <pre className="max-h-80 overflow-auto border-t border-[var(--color-outline-variant)] px-3 py-3 font-mono text-[11px] leading-5 text-[var(--color-on-surface)]">
            {JSON.stringify(part.value, null, 2)}
          </pre>
        </details>
      ))}
    </div>
  );
}

function ToolSchemaPreview({ value }: { value: unknown }) {
  if (!Array.isArray(value)) return null;
  const tools = value.filter((item): item is Record<string, unknown> => (
    item !== null && typeof item === "object" && !Array.isArray(item)
  ));
  if (tools.length === 0) return null;
  return (
    <div className="space-y-3">
      {tools.map((tool, index) => {
        const name = typeof tool.name === "string" ? tool.name : `Tool ${index + 1}`;
        const domainCode = typeof tool.domainCode === "string" ? tool.domainCode : null;
        const description = typeof tool.description === "string" ? tool.description : null;
        return (
          <article className="overflow-hidden rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]" key={`${name}-${index}`}>
            <div className="flex items-center justify-between gap-3 bg-[var(--color-surface-container-low)] px-3 py-2">
              <span className="min-w-0 truncate font-mono text-xs font-semibold text-[var(--color-on-surface)]">{name}</span>
              {domainCode ? (
                <span className="shrink-0 rounded bg-[var(--color-surface-container)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-on-surface-variant)]">{domainCode}</span>
              ) : null}
            </div>
            {description ? <div className="px-3 py-3 text-xs leading-5 text-[var(--color-on-surface)]">{description}</div> : null}
          </article>
        );
      })}
    </div>
  );
}

type ContextDetailTreeLabels = {
  arrayItem: (index: number) => string;
  arraySummary: (count: number) => string;
  emptyArray: string;
  emptyObject: string;
};

function ContextDetailTreeNode({
  value,
  label,
  labelKind = "key",
  depth = 0,
  labels,
}: {
  value: unknown;
  label?: string;
  labelKind?: "key" | "index";
  depth?: number;
  labels: ContextDetailTreeLabels;
}) {
  const isArray = Array.isArray(value);
  const isObject = value !== null && typeof value === "object" && !isArray;
  const entries: [string, unknown][] = isArray
    ? value.map((item, index) => [String(index), item])
    : isObject
      ? Object.entries(value as Record<string, unknown>)
      : [];
  const isContainer = isArray || isObject;
  const [expanded, setExpanded] = useState(depth < 2);
  const visibleLabel = labelKind === "index" && label != null
    ? labels.arrayItem(Number(label) + 1)
    : label;
  const labelContent = visibleLabel == null ? null : (
    <span className="break-all text-[var(--color-primary)]">
      {visibleLabel}{labelKind === "key" ? ":" : ""}
    </span>
  );

  if (!isContainer || entries.length === 0) {
    const renderedValue = isContainer
      ? (isArray ? labels.emptyArray : labels.emptyObject)
      : JSON.stringify(value) ?? String(value);
    return (
      <div className="flex min-w-0 items-start gap-1.5 py-0.5">
        <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-[var(--color-outline-variant)]" aria-hidden="true" />
        <div className="min-w-0 break-all">
          {labelContent ? <>{labelContent} </> : null}
          <span className={cn(
            value === null ? "text-[var(--color-on-surface-variant)]" : "text-[var(--color-on-surface)]",
            typeof value === "string" && "text-[var(--color-primary)]",
          )}>
            {renderedValue}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="min-w-0">
      <button
        aria-expanded={expanded}
        className="group flex min-w-0 items-center gap-1.5 rounded px-1 py-0.5 text-left outline-none transition-colors hover:text-[var(--color-primary)] focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
        type="button"
        onClick={() => setExpanded((current) => !current)}
      >
        {expanded ? <ChevronDown className="size-3.5 shrink-0 text-[var(--color-on-surface-variant)]" /> : <ChevronRight className="size-3.5 shrink-0 text-[var(--color-on-surface-variant)]" />}
        {labelContent}
        {isArray ? <span className="text-[var(--color-on-surface-variant)]">{labels.arraySummary(entries.length)}</span> : null}
      </button>
      {expanded ? (
        <div className="ml-2 border-l border-[var(--color-outline-variant)] py-0.5 pl-3">
          {entries.map(([entryKey, entryValue]) => (
            <ContextDetailTreeNode
              depth={depth + 1}
              key={entryKey}
              label={entryKey}
              labelKind={isArray ? "index" : "key"}
              labels={labels}
              value={entryValue}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function ContextMemoryBlockRow({ block, sessionCode }: { block: ContextMemoryBlockItem; sessionCode: string }) {
  const { t } = useTranslation("pages");
  const navigate = useNavigate();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [detail, setDetail] = useState<unknown>(null);
  const [detailViewMode, setDetailViewMode] = useState<"tree" | "raw">("tree");
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState(false);
  const colors = contextBlockColorClasses[block.color];
  const percent = contextPercent(block.tokens, block.limitTokens);
  const fallbackPercent = block.limitTokens == null && block.tokens > 0 ? 40 : percent;
  useEffect(() => {
    if (!dialogOpen || !block.detailCode) return;
    const controller = new AbortController();
    setLoading(true);
    setLoadError(false);
    setDetail(null);
    void requestApi<unknown>(chatApiEndpoints.getChatContextDetail, {
      queryParams: { sessionCode, blockCode: block.detailCode },
      signal: controller.signal,
    }).then(setDetail).catch(() => {
      if (!controller.signal.aborted) setLoadError(true);
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [block.detailCode, dialogOpen, sessionCode]);
  let parsedDetail = detail;
  let isStructuredDetail = detail !== null && typeof detail === "object";
  if (typeof detail === "string") {
    try {
      parsedDetail = JSON.parse(detail) as unknown;
      isStructuredDetail = parsedDetail !== null && typeof parsedDetail === "object";
    } catch {
      isStructuredDetail = false;
    }
  }
  const rawDetail = detail == null
    ? ""
    : typeof detail === "string"
      ? (isStructuredDetail ? JSON.stringify(parsedDetail, null, 2) : detail)
      : JSON.stringify(detail, null, 2) ?? String(detail);
  const promptMessages = block.detailCode === "systemPrompt"
    ? contextPromptPrefixMessages(parsedDetail)
    : null;
  const detailTreeLabels: ContextDetailTreeLabels = {
    arrayItem: (index) => t("chat.shortTermMemory.contextPanel.detailArrayItem", { index }),
    arraySummary: (count) => t("chat.shortTermMemory.contextPanel.detailArraySummary", { total: count }),
    emptyArray: t("chat.shortTermMemory.contextPanel.detailEmptyArray"),
    emptyObject: t("chat.shortTermMemory.contextPanel.detailEmptyObject"),
  };
  const contents = (
    <>
      <div className="flex min-w-0 items-baseline justify-between gap-[12px]">
        <div className="flex min-w-0 items-baseline gap-1">
          <div className="truncate text-xs font-semibold text-[var(--color-on-surface)] transition-colors group-hover:text-[var(--color-primary)]">{block.name}</div>
          {block.stat ? (
            <div className="shrink-0 text-[10px] text-[var(--color-on-surface-variant)]">· {block.stat}</div>
          ) : null}
        </div>
        <div className="shrink-0 font-mono text-[11px] text-[var(--color-on-surface)]">
          {formatTokenLimit(block.tokens, block.limitTokens)}
        </div>
      </div>
      <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
        <div className={cn("h-full rounded-full", colors.fill)} style={{ width: `${fallbackPercent}%` }} />
      </div>
    </>
  );
  return (
    <div className="py-2 first:pt-0 last:pb-0">
      {block.detailCode ? (
        <button className="group w-full cursor-pointer rounded text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]" type="button" onClick={() => setDialogOpen(true)}>
          {contents}
        </button>
      ) : contents}
      <Dialog open={dialogOpen} onOpenChange={(open) => {
        setDialogOpen(open);
        if (open) setDetailViewMode("tree");
      }}>
        <DialogContent className="grid max-h-[80vh] max-w-2xl grid-rows-[auto_minmax(0,1fr)] gap-3 overflow-hidden">
          <div className="flex min-w-0 items-center justify-between gap-3 pr-8">
            <h2 className="truncate text-sm font-semibold text-[var(--color-on-surface)]">{block.name}</h2>
            {isStructuredDetail ? (
              <div className="flex shrink-0 items-center gap-1">
                <CopyValueButton
                  ariaLabel={t("chat.shortTermMemory.contextPanel.copyRawJson")}
                  title={t("chat.shortTermMemory.contextPanel.copyRawJson")}
                  value={rawDetail}
                />
                <div className="flex items-center rounded-md bg-[var(--color-surface-container-low)] p-0.5">
                <button
                  aria-pressed={detailViewMode === "tree"}
                  className={cn("rounded px-2 py-1 text-xs transition-colors", detailViewMode === "tree" ? "bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] shadow-sm" : "text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]")}
                  type="button"
                  onClick={() => setDetailViewMode("tree")}
                >
                  {t("chat.shortTermMemory.contextPanel.detailViewTree")}
                </button>
                <button
                  aria-pressed={detailViewMode === "raw"}
                  className={cn("rounded px-2 py-1 text-xs transition-colors", detailViewMode === "raw" ? "bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] shadow-sm" : "text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]")}
                  type="button"
                  onClick={() => setDetailViewMode("raw")}
                >
                  {t("chat.shortTermMemory.contextPanel.detailViewRaw")}
                </button>
                </div>
              </div>
            ) : null}
          </div>
          <div className="min-h-0 overflow-auto rounded-md bg-[var(--color-surface-container-lowest)] p-3">
            {loading ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("chat.shortTermMemory.contextPanel.detailLoading")}</p>
              : loadError ? <p className="text-sm text-[var(--color-error)]">{t("chat.shortTermMemory.contextPanel.detailFailed")}</p>
                : block.detailCode === "toolSchema" && isStructuredDetail && detailViewMode === "tree"
                  ? <ToolSchemaPreview value={parsedDetail} />
                  : block.detailCode === "systemPrompt" && isStructuredDetail && !promptMessages && detailViewMode === "tree"
                  ? <div className="min-w-0 font-mono text-xs leading-5"><ContextDetailTreeNode labels={detailTreeLabels} value={parsedDetail} /></div>
                  : block.detailCode === "systemPrompt" && promptMessages && detailViewMode === "tree"
                    ? promptMessages.length > 0
                      ? <div className="space-y-3">
                          {promptMessages.map((message, index) => (
                            <article className="overflow-hidden rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]" key={`${index}-${message.role}`}>
                              <div className="flex items-center justify-between gap-3 bg-[var(--color-surface-container-low)] px-3 py-2">
                                <span className="text-xs font-semibold text-[var(--color-on-surface)]">
                                  {t("chat.shortTermMemory.contextPanel.detailPromptMessage", { index: index + 1 })}
                                </span>
                                <div className="flex min-w-0 items-center justify-end gap-1.5">
                                  {message.promptCode ? (
                                    <button
                                      className="shrink-0 rounded-sm px-2 py-1 text-[11px] font-medium text-[var(--color-primary)] transition-colors hover:bg-[var(--color-primary)] hover:text-[var(--color-on-primary)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
                                      type="button"
                                      onClick={() => {
                                        const promptPath = `/prompts/detail?${new URLSearchParams({ promptCode: message.promptCode }).toString()}`;
                                        const tabId = useTabStore.getState().addTab({
                                          title: t("chat.shortTermMemory.contextPanel.detailPromptManagement"),
                                          path: promptPath,
                                          closable: true,
                                          activeNav: `prompt-detail:${message.promptCode}`,
                                        });
                                        useTabStore.getState().updateTab(tabId, { path: promptPath });
                                        setDialogOpen(false);
                                        navigate(promptPath);
                                      }}
                                    >
                                      {t("chat.shortTermMemory.contextPanel.detailPromptManagement")}
                                    </button>
                                  ) : null}
                                </div>
                              </div>
                              <div className="px-3 py-3">
                                <ContextPromptMessageContent content={message.content} />
                              </div>
                            </article>
                          ))}
                        </div>
                      : <p className="text-sm text-[var(--color-on-surface-variant)]">{t("chat.shortTermMemory.contextPanel.detailPromptEmpty")}</p>
                  : isStructuredDetail && (detailViewMode === "tree" || detailViewMode === "raw")
                    ? <div className="min-w-0 font-mono text-xs leading-5"><ContextDetailTreeNode labels={detailTreeLabels} value={parsedDetail} /></div>
                  : <pre className="whitespace-pre-wrap break-all font-mono text-xs leading-5 text-[var(--color-on-surface)]">{rawDetail}</pre>}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ProviderUsagePanel({
  events,
  fallbackSessionUsage,
  usageAudit,
}: {
  events: TranscriptAguiEvent[];
  fallbackSessionUsage: { input: number | null; output: number | null; total: number | null };
  usageAudit: ChatUsageAuditSnapshot | null;
}) {
  const { t } = useTranslation("pages");
  const providerUsage = usageAudit?.providerUsage ?? null;
  const sessionUsage = sessionProviderUsageFromEvents(events);
  const requestEstimate = usageAudit?.requestEstimate ?? null;
  const sessionInput = sessionUsage?.inputTokens ?? fallbackSessionUsage.input;

  return (
    <section className="space-y-3 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3">
      <div>
        <h3 className="text-xs font-semibold uppercase text-[var(--color-on-surface-variant)]">
          {t("chat.shortTermMemory.contextSections.providerUsage")}
        </h3>
      </div>
      <div>
        <h4 className="mb-1.5 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
          {t("chat.shortTermMemory.contextPanel.currentSessionUsage")}
        </h4>
        <div className="grid grid-cols-3 gap-1.5">
          <ContextStat label={t("chat.shortTermMemory.contextPanel.actualInput")} value={formatCompactTokenCount(sessionInput)} />
          <ContextStat label={t("chat.shortTermMemory.contextPanel.cached")} value={formatCompactTokenCount(sessionUsage?.cachedInputTokens)} />
          <ContextStat label={t("chat.shortTermMemory.contextPanel.reasoning")} value={formatCompactTokenCount(sessionUsage?.reasoningTokens)} />
        </div>
      </div>
      <div>
        <h4 className="mb-1.5 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
          {t("chat.shortTermMemory.contextPanel.latestRequestUsage")}
        </h4>
        <div className="grid grid-cols-3 gap-1.5">
          <ContextStat label={t("chat.shortTermMemory.contextPanel.actualInput")} value={formatCompactTokenCount(providerUsage?.inputTokens)} />
          <ContextStat label={t("chat.shortTermMemory.contextPanel.cached")} value={formatCompactTokenCount(providerUsage?.cachedInputTokens)} />
          <ContextStat label={t("chat.shortTermMemory.contextPanel.reasoning")} value={formatCompactTokenCount(providerUsage?.reasoningTokens)} />
        </div>
        <p className="mt-2 rounded-md bg-[var(--color-surface-container-low)] px-2 py-1.5 text-[10px] leading-4 text-[var(--color-on-surface-variant)]">
          {requestEstimate ? (
            <>
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
            </>
          ) : (
            t("chat.shortTermMemory.contextPanel.providerUnavailable")
          )}
        </p>
      </div>
    </section>
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
  const syncedGoalDraftSourceRef = useRef<string | null>(null);
  const goalDraftSource = useMemo(() => JSON.stringify({
    goalCode: goal?.goal.goalCode ?? "",
    maxRuns: goal?.goal.maxRuns ?? 0,
    objective: goal?.goal.objectiveMarkdown ?? "",
    sessionCode: sessionCode ?? "",
  }), [goal?.goal.goalCode, goal?.goal.maxRuns, goal?.goal.objectiveMarkdown, sessionCode]);
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
    if (syncedGoalDraftSourceRef.current === goalDraftSource) return;

    syncedGoalDraftSourceRef.current = goalDraftSource;
    setGoalDraft(goal?.goal.objectiveMarkdown ?? "");
    setGoalMaxRunsDraft(goal ? String(goal.goal.maxRuns) : "20");
  }, [creatingGoal, editingGoal, goal, goalDraftSource]);

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
            <Badge className="h-5 rounded px-1.5 text-[11px]" variant={goalStatusTone(goal.goal.status)}>
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
          <Badge className="h-5 rounded px-1.5 text-[11px]" variant="secondary">
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
                className="grid min-w-0 grid-cols-[0.5rem_minmax(0,1fr)_auto_auto] items-center gap-[12px] py-1 text-xs"
                title={todo.content}
              >
                <span className={cn("size-2 rounded-full", todoStatusDotClass(todo.status))} aria-hidden="true" />
                <span className="min-w-0 truncate text-sm text-[var(--color-on-surface)]">{todo.content}</span>
                <Badge className="h-5 rounded px-1.5 text-[11px]" variant={todoStatusTone(todo.status)}>
                  {t(`chat.shortTermMemory.todoStatuses.${todo.status}`)}
                </Badge>
                <Badge className="h-5 rounded px-1.5 text-[11px]" variant={todoPriorityTone(todo.priority)}>
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
  events: TranscriptAguiEvent[];
  goal?: ChatSessionGoalView | null;
  latestCompaction?: ChatSessionCompaction | null;
  latestCompactionStatus?: QueryStatus;
  latestRequestEstimate?: ChatTokenEstimateSnapshot | null;
  latestUsageAudit?: ChatUsageAuditSnapshot | null;
  messages: ChatMessage[];
  model?: ChatModelOption | null;
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
  const runtimeNotice = latestShortTermMemoryRuntimeNotice(events);
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
    <section className="space-y-3 text-sm">
      <div className="space-y-2 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3" data-testid="context-short-term-memory-card">
        <ContextOverviewPanel
          breakdown={budgetBreakdown}
          compactionBusy={compactionBusy || latestCompactionStatus?.loading}
          fallbackWindowLimit={fallbackWindowLimit}
          modelLabel={modelLabel}
          onCreateCompaction={onCreateCompaction}
          requestEstimate={requestEstimate}
          session={session}
        />
        <div className="pt-2">
          {runtimeNotice ? <ContextRuntimeNoticePanel notice={runtimeNotice} /> : null}
          <ContextStructurePanel blocks={contextBlocks} sessionCode={session?.sessionCode ?? ""} />
        </div>
      </div>
      {latestCompactionStatus?.error ? (
        <QueryStatusLine
          errorText={t("chat.shortTermMemory.loadFailed")}
          loadingText={t("chat.shortTermMemory.loadingCheckpoint")}
          retryText={t("chat.shortTermMemory.retry")}
          status={latestCompactionStatus}
        />
      ) : null}
      <ProviderUsagePanel events={events} fallbackSessionUsage={usage} usageAudit={usageAudit} />
    </section>
  );
}

function ChatSessionRightPanel({
  active,
  snapshot,
  onCancelGoal,
  onClearGoal,
  onCreateCompaction,
  onCreateGoal,
  onPauseGoal,
  onResumeGoal,
  onUpdateGoal,
}: {
  active: boolean;
  snapshot: ChatRightPanelSnapshot;
  onCancelGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onClearGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onCreateCompaction: () => Promise<void> | void;
  onCreateGoal: (input: SetChatSessionGoalInput) => Promise<boolean> | boolean;
  onPauseGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onResumeGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onUpdateGoal: (input: UpdateChatSessionGoalInput) => Promise<boolean> | boolean;
}) {
  const { t } = useTranslation("pages");
  const [activeTab, setActiveTab] = useState("workspace");
  const sessionCode = snapshot.session.sessionCode;
  const tabs = useMemo<EditableTab[]>(() => [
    {
      key: "workspace",
      label: (
        <>
          <WorkspaceTabState goal={snapshot.goal} running={snapshot.workspaceGoalRunning} />
          <span>{t("chat.shortTermMemory.workspace")}</span>
        </>
      ),
      closable: false,
      children: (
        <div className="min-h-0 overflow-y-auto">
          <div className="m-4">
            <RightWorkspaceSidebar
              goal={snapshot.goal}
              goalBusy={snapshot.goalBusy}
              goalStatus={snapshot.goalStatus}
              onCancelGoal={onCancelGoal}
              onClearGoal={onClearGoal}
              onCreateGoal={onCreateGoal}
              onPauseGoal={onPauseGoal}
              onResumeGoal={onResumeGoal}
              onUpdateGoal={onUpdateGoal}
              sessionCode={sessionCode}
              todos={snapshot.todos}
              todosStatus={snapshot.todosStatus}
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
        <div className="absolute inset-0 min-h-0 overflow-y-auto overscroll-y-contain">
          <div className="m-4">
            <RightContextSidebar
              events={snapshot.contextEvents}
              goal={snapshot.goal}
              latestCompaction={snapshot.latestCompaction}
              latestCompactionStatus={snapshot.latestCompactionStatus}
              latestRequestEstimate={snapshot.latestRequestEstimate}
              latestUsageAudit={snapshot.latestUsageAudit}
              messages={snapshot.messages}
              model={snapshot.model}
              compactionBusy={snapshot.compactionBusy}
              onCreateCompaction={onCreateCompaction}
              selectedModel={snapshot.selectedModel}
              session={snapshot.session}
              todos={snapshot.todos}
            />
          </div>
        </div>
      ),
    },
  ], [
    onCancelGoal,
    onClearGoal,
    onCreateCompaction,
    onCreateGoal,
    onPauseGoal,
    onResumeGoal,
    onUpdateGoal,
    sessionCode,
    snapshot,
    t,
  ]);

  return (
    <div
      aria-hidden={!active}
      className={active ? "relative h-full w-full" : "absolute inset-0 h-full w-full invisible pointer-events-none"}
      data-session-code={sessionCode}
    >
      <EditableTabs
        activeKey={activeTab}
        className="h-full min-h-0"
        defaultActiveKey="workspace"
        hideAdd
        items={tabs}
        keepMounted
        onChange={(key) => setActiveTab(key ?? "workspace")}
      />
    </div>
  );
}

function ChatSessionRightPanelCache({
  activeSessionCode,
  snapshots,
  onCancelGoal,
  onClearGoal,
  onCreateCompaction,
  onCreateGoal,
  onPauseGoal,
  onResumeGoal,
  onUpdateGoal,
}: {
  activeSessionCode: string;
  snapshots: Readonly<Record<string, ChatRightPanelSnapshot>>;
  onCancelGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onClearGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onCreateCompaction: () => Promise<void> | void;
  onCreateGoal: (input: SetChatSessionGoalInput) => Promise<boolean> | boolean;
  onPauseGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onResumeGoal: (input: ChatSessionGoalCommand) => Promise<boolean> | boolean;
  onUpdateGoal: (input: UpdateChatSessionGoalInput) => Promise<boolean> | boolean;
}) {
  return (
    <div className="relative h-full w-full overflow-hidden">
      {Object.values(snapshots).map((snapshot) => (
        <ChatSessionRightPanel
          active={snapshot.session.sessionCode === activeSessionCode}
          key={snapshot.session.sessionCode}
          snapshot={snapshot}
          onCancelGoal={onCancelGoal}
          onClearGoal={onClearGoal}
          onCreateCompaction={onCreateCompaction}
          onCreateGoal={onCreateGoal}
          onPauseGoal={onPauseGoal}
          onResumeGoal={onResumeGoal}
          onUpdateGoal={onUpdateGoal}
        />
      ))}
    </div>
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
      <div className="flex min-w-0 items-center justify-between gap-[12px]">
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
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const { t } = useTranslation("pages");
  const selectedModel = models.find((model) => model.optionCode === selectedModelCode) ?? null;
  const modelDisplayName = (model: ChatModelOption) => (
    isPersonAiAutoModel(model)
      ? t("chat.management.modelSelector.autoName")
      : model.displayName
  );
  const outputMultiplier = (model: ChatModelOption) => (
    model.providerCode === "PersonAi" && !isPersonAiAutoModel(model)
      ? formatModelMultiplier(model.outputCoefficient)
      : null
  );
  const modelTitle = (model: ChatModelOption) => {
    return [
      modelDisplayName(model),
      model.upstreamVendorName,
    ].filter(Boolean).join("\n");
  };
  const filteredModels = useMemo(
    () => models.filter((model) => model.enabled && fuzzyModelMatch(searchQuery, model)),
    [models, searchQuery],
  );
  const autoModels = useMemo(
    () => filteredModels.filter(isPersonAiAutoModel),
    [filteredModels],
  );
  const groupedModels = useMemo(() => {
    const groups = new Map<string, { label: string; models: ChatModelOption[] }>();
    filteredModels
      .filter((model) => !isPersonAiAutoModel(model))
      .forEach((model) => {
        const qualityType = model.qualityType ?? "High";
        const vendor = model.upstreamVendorName ?? model.vendor ?? t("modelProviders.values.unknown");
        const personAiGroup = model.providerCode === "PersonAi";
        const key = personAiGroup
          ? `${model.providerCode}:${qualityType}:${vendor}`
          : model.accessPlatform ?? "platform";
        const label = personAiGroup
          ? t("chat.management.modelSelector.qualityGroup", {
              quality: t(`chat.management.modelSelector.quality.${qualityType}`),
              vendor,
            })
          : model.accessPlatform ?? t("chat.management.modelSelector.platformModels");
        const group = groups.get(key) ?? { label, models: [] };
        group.models.push(model);
        groups.set(key, group);
      });
    return Array.from(groups.entries()).map(([key, group]) => ({
      key,
      label: group.label,
      models: group.models.sort((left, right) => left.displayName.localeCompare(right.displayName)),
    }));
  }, [filteredModels, t]);
  const disabled = loading || models.length === 0;
  const selectedModelLabel = loading
    ? t("chat.management.modelSelector.loading")
    : selectedModel
      ? modelDisplayName(selectedModel)
      : emptyLabel;

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
      <DropdownMenu open={open} onOpenChange={setOpen}>
        <DropdownMenuTrigger
          aria-label={t("chat.management.modelSelector.selectAria")}
          className="inline-flex h-8 w-full min-w-0 items-center gap-[12px] rounded-md border-0 bg-transparent px-2 text-xs text-[var(--color-on-surface-variant)] shadow-none transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
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
              aria-label={t("chat.management.modelSelector.searchAria")}
              className="h-8 w-full rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 text-xs text-[var(--color-on-surface)] outline-none focus:border-[var(--color-focus-ring)]"
              placeholder={t("chat.management.modelSelector.searchPlaceholder")}
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              onKeyDown={(event) => {
                event.stopPropagation();
                if (event.key !== "Enter") return;
                const firstModel = filteredModels.find((model) => model.enabled);
                if (!firstModel) return;
                event.preventDefault();
                onSelect(firstModel.optionCode);
                setOpen(false);
              }}
            />
          </div>
          {filteredModels.length ? (
            <DropdownMenuRadioGroup
              value={selectedModelCode}
              onValueChange={(modelCode) => onSelect(modelCode)}
            >
              {autoModels.map((model) => (
                <DropdownMenuRadioItem
                  className="w-full items-center px-3 py-2"
                  key={model.optionCode}
                  title={modelTitle(model)}
                  value={model.optionCode}
                >
                  <span className="min-w-0 truncate font-medium">
                    {modelDisplayName(model)}
                  </span>
                </DropdownMenuRadioItem>
              ))}
              {groupedModels.map((group) => (
                <div key={group.key} className="mb-1 last:mb-0">
                  <p className="px-3 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wide text-[var(--color-on-surface-variant)]">
                    {group.label}
                  </p>
                  {group.models.map((model) => (
                    <DropdownMenuRadioItem
                      className="w-full items-center px-3 py-2"
                      key={model.optionCode}
                      title={modelTitle(model)}
                      value={model.optionCode}
                    >
                      <span className="flex min-w-0 flex-1 items-center gap-2">
                        <span className="min-w-0 flex-1 truncate font-medium">
                          {modelDisplayName(model)}
                        </span>
                        {outputMultiplier(model) ? (
                          <span className="shrink-0 font-mono text-[10px] text-[var(--color-on-surface-variant)]">
                            {outputMultiplier(model)}×
                          </span>
                        ) : null}
                      </span>
                    </DropdownMenuRadioItem>
                  ))}
                </div>
              ))}
            </DropdownMenuRadioGroup>
          ) : (
            <p className="px-3 py-2 text-xs text-[var(--color-on-surface-variant)]">
              {t("chat.management.modelSelector.noMatch", { query: searchQuery.trim() })}
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
  const { t } = useTranslation("pages");
  const selectedAgent = agents.find((agent) => agent.agentCode === selectedAgentCode) ?? null;
  const disabled = loading || agents.length === 0 || Boolean(locked);

  return (
    <div className="relative min-w-0">
      <DropdownMenu open={open} onOpenChange={setOpen}>
        <DropdownMenuTrigger
          aria-label={t("chat.management.agentSelector.selectAria")}
          className="inline-flex h-8 w-full min-w-0 items-center gap-[12px] rounded-md border-0 bg-transparent px-2 text-xs text-[var(--color-on-surface-variant)] shadow-none transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
          disabled={disabled}
          title={locked ? t("chat.management.agentSelector.locked") : undefined}
        >
          <Bot aria-hidden="true" size={14} />
          <span className="truncate">{loading ? t("chat.management.agentSelector.loading") : selectedAgent?.name ?? t("chat.management.agentSelector.mainAgent")}</span>
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
  pendingUserInputSessionCodes,
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
  pendingUserInputSessionCodes: Set<string>;
  unseenSessionCodes: Set<string>;
}) {
  const [closedGroups, setClosedGroups] = useState<Set<string>>(() => new Set());
  const [editingSessionCode, setEditingSessionCode] = useState("");
  const [renameDraft, setRenameDraft] = useState("");
  const { t } = useTranslation("pages");
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
          "grid w-full grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-[12px] rounded-md px-3 py-2 text-left transition",
          activeSessionCode === session.sessionCode
            ? "bg-[var(--color-surface-container-high)]"
            : "hover:bg-[var(--color-surface-container-high)]",
        )}
        key={session.sessionCode}
      >
        {editing ? (
          <input
            autoFocus
            aria-label={t("chat.management.sidebar.renameAria", { title: session.title })}
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
          needsUserInput={pendingUserInputSessionCodes.has(session.sessionCode)}
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
              aria-label={t("chat.management.sidebar.newSession")}
              title={t("chat.management.sidebar.newSession")}
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
                aria-label={t("chat.management.sidebar.openAria", { title: session.title })}
                onClick={() => onSessionSelect(session, activeSourceTab)}
                onMouseEnter={(event) => showCollapsedTooltip(event.currentTarget, session)}
                onMouseLeave={hideCollapsedTooltip}
                onFocus={(event) => showCollapsedTooltip(event.currentTarget, session)}
                onBlur={hideCollapsedTooltip}
              >
                {pendingUserInputSessionCodes.has(session.sessionCode) ? (
                  <MessageCircleQuestionMark className="text-[var(--color-focus-ring)]" size={17} />
                ) : (
                  <MessageSquareText className={sessionAgentIconTone(session.agentCode)} size={17} />
                )}
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
            <div className="flex items-center gap-[12px]">
              <MessageSquareText size={17} />
              <h2 className="text-sm font-semibold">{t("chat.management.sidebar.sessions")}</h2>
            </div>
            <div className="flex items-center gap-1">
              <button
                className={iconButtonClass()}
                type="button"
                aria-label={t("chat.management.sidebar.newSession")}
                title={t("chat.management.sidebar.newSession")}
                disabled={creatingSession}
                onClick={onCreateSession}
              >
                <Plus size={17} />
              </button>
            </div>
          </div>
          <div className="h-[calc(100%-3.5rem)] overflow-y-auto px-3 py-4">
            <div className="mb-4 grid grid-cols-3 gap-1 rounded-md bg-[var(--color-surface-container)] p-1" role="tablist" aria-label={t("chat.management.sourceTabs.aria")}>
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
                    aria-label={t(tab.labelKey)}
                    aria-selected={activeSourceTab === tab.id}
                    title={t(tab.labelKey)}
                    onClick={() => onSourceTabSelect(tab.id)}
                  >
                    <TabIcon size={17} strokeWidth={2.2} />
                    <span className="sr-only">{t(tab.labelKey)}</span>
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
                    {t("chat.management.sidebar.noSessions")}
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
                {t("chat.management.sidebar.noSessions")}
              </p>
            ) : null}
          </div>
        </>
      )}
      <DraggleLine
        ariaLabel={collapsed ? t("chat.management.sidebar.expandHistory") : t("chat.management.sidebar.collapseHistory")}
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
  const { t } = useTranslation("pages");
  async function handleEditMessage() {
    if (!isUser || !message.content || !onEditMessage) return;
    onEditMessage(message.content);
  }

  return (
    <article className={cn("max-w-[920px] rounded-md bg-white px-4 py-3 shadow-[0_8px_22px_rgba(15,23,42,0.06)]", isUser ? "ml-auto bg-[var(--color-surface-container-lowest)]" : "")}>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-[12px]">
          {isUser ? <UserRound size={16} /> : <Bot size={16} />}
          <span className="truncate text-sm font-semibold">{actorName}</span>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {!isUser ? (
            <CopyValueButton
              ariaLabel={t("chat.management.bubble.copyRaw")}
              title={t("chat.management.bubble.copyRaw")}
              value={message.content}
            />
          ) : null}
          {isUser ? (
            <>
              <CopyValueButton
                ariaLabel={t("chat.management.bubble.copyRaw")}
                title={t("chat.management.bubble.copyRaw")}
                value={message.content}
              />
              <button
                aria-label={t("chat.management.bubble.editMessage")}
                className="inline-flex size-6 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-40"
                disabled={!message.content || !onEditMessage}
                title={t("chat.management.bubble.editMessage")}
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
          ariaLabel={t("chat.management.bubble.streamingAria")}
          className="py-0"
          content={message.content}
          emptyMessage={t("chat.management.bubble.waitingOutput")}
        />
      )}
    </article>
  );
}

function ReasoningUserCard({ card }: { card: ReasoningWorkCard }) {
  const { t } = useTranslation("pages");
  const detail = card.content || card.summary;
  const hasContent = detail.trim().length > 0;
  const initializingHint = !card.ended && !hasContent && card.runCode === "pending"
    ? t("chat.management.reasoning.initializing")
    : "";
  const [expanded, setExpanded] = useState(() => !card.ended);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const nestedToolCards = card.toolCards.filter((toolCard) => toolCard.started);
  const hasRunningTool = nestedToolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result);
  const shouldStayExpanded = !card.ended || hasRunningTool;
  const reasoningPreview = deriveReasoningPreview(
    card.summary || reasoningLastSentence(card.content),
  );
  const headerSummary = initializingHint || reasoningPreview || (card.ended ? t("chat.management.reasoning.captured") : t("chat.management.reasoning.thinking"));

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
      aria-label={t("chat.management.reasoning.aria")}
      className="text-sm"
    >
      <button
        className="group flex w-full items-start justify-between gap-3 rounded-md py-1 text-left text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((current) => !current)}
      >
        <div className="flex min-w-0 items-start gap-[12px]">
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
                ariaLabel={t("chat.management.reasoning.streamingAria")}
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
  const { t } = useTranslation("pages");
  const complete = card.ended || Boolean(card.result);
  const hasResultPayload = Boolean(card.result);
  const result = card.result || (complete ? t("chat.management.tool.noResult") : "");
  const [expanded, setExpanded] = useState(() => !complete);
  const logRef = useRef<HTMLDivElement | null>(null);
  const running = card.started && !complete;
  const hasParameters = hasToolParameterPayload(card.args);
  const parametersText = hasParameters ? formatJsonBlockValue(card.args) : "";
  const resultText = hasResultPayload ? formatJsonBlockValue(card.result) : "";
  const toolLabel = toolSummaryLabel(card.toolName, card.args, t);
  const commandLine = toolCommandLinePreview(card.toolName, card.args);
  const nestedToolLabel = commandLine
    ? t("chat.management.tool.commandSummary", { command: commandLine })
    : `${complete ? t("chat.management.tool.ranPrefix") : t("chat.management.tool.runningPrefix")} ${toolLabel.replace(/^(Run|Edit|运行|编辑)\s+/i, "")}`;
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
        <div className="flex min-w-0 items-center gap-[12px]">
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
            <div className="flex min-w-0 items-center gap-[12px]">
              <span className={cn("truncate", nested ? "font-normal" : "font-semibold")}>
                {nested ? nestedToolLabel : toolLabel}
              </span>
              {nested ? null : <Badge variant={complete ? "info" : "success"}>{complete ? t("chat.management.tool.badgeDone") : t("chat.management.tool.badgeRunning")}</Badge>}
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
                <div className="flex items-center justify-between gap-[12px]">
                  <button
                    aria-expanded={parametersExpanded}
                    className="inline-flex min-w-0 items-center gap-1 rounded py-0.5 pr-1 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
                    type="button"
                    onClick={() => setParametersExpanded((current) => !current)}
                  >
                    {parametersExpanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                    <span>{t("chat.management.tool.parameters")}</span>
                  </button>
                  <CopyValueButton ariaLabel={t("chat.management.tool.copyParameters")} title={t("chat.management.tool.copyParameters")} value={parametersText} />
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
                <div className="flex items-center justify-between gap-[12px]">
                  <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{t("chat.management.tool.result")}</h3>
                  <CopyValueButton ariaLabel={t("chat.management.tool.copyResult")} title={t("chat.management.tool.copyResult")} value={resultText} />
                </div>
                <div className="max-h-64 overflow-y-auto">
                  {result ? <JsonBlock value={result} /> : (
                    <div className="rounded-md bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                      {t("chat.management.tool.noResult")}
                    </div>
                  )}
                </div>
              </section>
            ) : (
              <section className="flex min-h-0 flex-col gap-1">
                <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{t("chat.management.tool.log")}</h3>
                <div
                  ref={logRef}
                  className="max-h-64 space-y-1 overflow-y-auto rounded-md bg-[var(--color-surface-container-lowest)] p-3 font-mono text-xs leading-5 text-[var(--color-on-surface)]"
                >
                  {card.logs.length ? card.logs.map((line, index) => (
                    <p className="whitespace-pre-wrap" key={`${card.id}-log-${index}`}>{line}</p>
                  )) : <p className="text-[var(--color-on-surface-variant)]">{t("chat.management.tool.waitingActivity")}</p>}
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
  const { t } = useTranslation("pages");
  const toolCards = card.toolCards.filter((toolCard) => toolCard.started);
  const hasRunningTool = toolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result);
  const toolCount = toolCards.length;
  const [expanded, setExpanded] = useState(() => hasRunningTool);

  useEffect(() => {
    setExpanded(hasRunningTool);
  }, [hasRunningTool]);

  if (!toolCount) return null;

  return (
    <section className="text-sm" aria-label={t("chat.management.tool.stepsAria")}>
      <button
        aria-expanded={expanded}
        className="group flex w-full items-center justify-between gap-3 rounded-md py-1 text-left text-[var(--color-on-surface-variant)] transition hover:text-[var(--color-on-surface)] focus:outline-none focus-visible:bg-[var(--color-surface-container-low)] focus-visible:text-[var(--color-on-surface)]"
        type="button"
        onClick={() => setExpanded((current) => !current)}
      >
        <span className="flex min-w-0 items-center gap-[12px]">
          <span className="inline-flex size-5 shrink-0 items-center justify-center rounded text-[var(--color-on-surface-variant)] transition group-hover:bg-[var(--color-surface-container-low)] group-hover:text-[var(--color-on-surface)]">
            <Terminal className={hasRunningTool ? "animate-pulse" : undefined} size={15} />
          </span>
          <span className="truncate font-medium">
            {t("chat.management.tool.ranCommands", { count: toolCount })}
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
  const { t } = useTranslation("pages");
  if (!cards.length) return null;

  return (
    <section className="space-y-2 py-1" aria-label={t("chat.workRail.ariaLabel")}>
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

function AguiWaitingForModelRail() {
  const { t } = useTranslation("pages");
  return (
    <section className="max-w-[920px]" aria-label={t("chat.management.rail.waitingModelAria")}>
      <div className="flex w-full items-center gap-3 py-1 text-xs font-medium text-[var(--color-on-surface-variant)]">
        <span className="h-px flex-1 bg-[var(--color-outline-variant)]" />
        <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
          <LoaderCircle className="animate-spin" size={14} />
          {t("chat.management.rail.waitingModel")}
        </span>
        <span className="h-px flex-1 bg-[var(--color-outline-variant)]" />
      </div>
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
  const { i18n, t } = useTranslation("pages");
  const userInputLabels = useMemo<ChatUserInputLabels>(() => ({
    ariaLabel: t("chat.userInput.ariaLabel"),
    progress: (current, total) => t("chat.userInput.progress", { current, total }),
    back: t("chat.userInput.back"),
    next: t("chat.userInput.next"),
    continue: t("chat.userInput.continue"),
    continuing: t("chat.userInput.continuing"),
    otherOptionLabel: t("chat.userInput.otherOptionLabel"),
    otherOptionDescription: t("chat.userInput.otherOptionDescription"),
    skipOptionLabel: t("chat.userInput.skipOptionLabel"),
    skipOptionDescription: t("chat.userInput.skipOptionDescription"),
    otherAnswerPlaceholder: t("chat.userInput.otherAnswerPlaceholder"),
    otherAnswerAriaLabel: (index) => t("chat.userInput.otherAnswerAriaLabel", { index }),
  }), [i18n.language, t]);
  const workRailLabels = useChatWorkRailLabels();
  const { session: authSession } = useAuthSession();
  const { preferences: aguiWorkReplayPreferences } = useAguiWorkReplayPreferences();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const chatRuntimeAdapter = useMemo(
    () => getServiceDataSource("chat") === "mock"
      ? createMockChatAdapter()
      : createInternalChatAdapter({ getAccessToken: getApiAccessToken }),
    [],
  );
  const [activeChatDataRefreshKey, setActiveChatDataRefreshKey] = useState(0);
  const [activeChatReplayRefreshKey, setActiveChatReplayRefreshKey] = useState(0);
  const [activeRunRefreshKey, setActiveRunRefreshKey] = useState(0);
  const [shortTermMemoryRefreshKey, setShortTermMemoryRefreshKey] = useState(0);
  const groups = useChatSessionGroupList(activeWorkspaceCode, activeChatDataRefreshKey);
  const [sessionGroups, setSessionGroups] = useState<ChatSessionGroup[]>([]);
  const sessionGroupsRef = useRef(sessionGroups);
  sessionGroupsRef.current = sessionGroups;
  const pendingUserInputSessionCodes = useMemo(
    () => new Set(
      sessionGroups.flatMap((group) => group.pendingUserInputSessionCodes ?? []),
    ),
    [sessionGroups],
  );
  const [activeSessionCode, setActiveSessionCode] = useState("");
  const [activeSourceTab, setActiveSourceTab] = useState<ChatSessionSourceTab>(() => (
    requestedSourceTab ?? defaultChatSessionSourceTab
  ));
  const [userSidebarCollapsed, setUserSidebarCollapsed] = useState(false);
  const [autoSidebarCollapsed, setAutoSidebarCollapsed] = useState(false);
  const sidebarCollapsed = userSidebarCollapsed || autoSidebarCollapsed;
  const mainRef = useRef<HTMLDivElement>(null);
  const [composerDraftsBySessionCode, setComposerDraftsBySessionCode] = useState<Record<string, string>>({});
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
  const [liveAguiPendingAnchorMessageCodesBySessionCode, setLiveAguiPendingAnchorMessageCodesBySessionCode] = useState<Record<string, string>>({});
  const [latestRequestEstimatesBySessionCode, setLatestRequestEstimatesBySessionCode] = useState<Record<string, ChatTokenEstimateSnapshot>>({});
  const [latestUsageAuditsBySessionCode, setLatestUsageAuditsBySessionCode] = useState<Record<string, ChatUsageAuditSnapshot>>({});
  const [sending, setSending] = useState(false);
  const [userInputRequests, setUserInputRequests] = useState<ChatUserInputRequest[]>([]);
  const [userInputRequest, setUserInputRequest] = useState<ChatUserInputRequest | null>(null);
  const [userInputAnswerPending, setUserInputAnswerPending] = useState(false);
  const [steering, setSteering] = useState(false);
  const [activeRunCode, setActiveRunCode] = useState("");
  const [activeRunSupportsSteer, setActiveRunSupportsSteer] = useState<boolean | null>(null);
  const [queuedSubmissions, setQueuedSubmissions] = useState<ChatQueuedSubmission[]>([]);
  const [creatingSession, setCreatingSession] = useState(false);
  const [chatModels, setChatModels] = useState<ModelSelectOption[]>([]);
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
  const [pendingAttachmentsBySessionCode, setPendingAttachmentsBySessionCode] = useState<
    Record<string, ChatMessageAttachment[]>
  >({});
  const [attachmentUploadPending, setAttachmentUploadPending] = useState(false);
  const [rightPanelSnapshots, setRightPanelSnapshots] = useState<
    Record<string, ChatRightPanelSnapshot>
  >({});
  const desktopBridge = useMemo(() => new DesktopBridge(), []);

  const composerSessionCode = activeSession?.sessionCode ?? "";
  const composer = composerSessionCode
    ? composerDraftsBySessionCode[composerSessionCode] ?? ""
    : "";
  const pendingAttachments = composerSessionCode
    ? pendingAttachmentsBySessionCode[composerSessionCode] ?? []
    : [];
  const setComposer = useCallback(
    (value: string | ((current: string) => string)) => {
      if (!composerSessionCode) return;
      setComposerDraftsBySessionCode((current) => {
        const previous = current[composerSessionCode] ?? "";
        const nextValue = typeof value === "function" ? value(previous) : value;
        if (nextValue === previous) return current;
        return { ...current, [composerSessionCode]: nextValue };
      });
    },
    [composerSessionCode],
  );
  const setPendingAttachments = useCallback(
    (
      value: ChatMessageAttachment[] | ((current: ChatMessageAttachment[]) => ChatMessageAttachment[]),
    ) => {
      if (!composerSessionCode) return;
      setPendingAttachmentsBySessionCode((current) => {
        const previous = current[composerSessionCode] ?? [];
        const nextValue = typeof value === "function" ? value(previous) : value;
        if (nextValue === previous) return current;
        return { ...current, [composerSessionCode]: nextValue };
      });
    },
    [composerSessionCode],
  );

  const { openTab } = useRightPanelTabs(activeSession?.sessionCode);

  const openMarkdownInternalLink = useCallback((url: string) => {
    const normalizedUrl = url.trim();
    if (!normalizedUrl) return;
    openTab({
      key: `markdown-page:${normalizedUrl}`,
      stableId: `markdown-page:${normalizedUrl}`,
      label: normalizedUrl,
      url: normalizedUrl,
      children: null,
    });
  }, [openTab]);
  const openMarkdownExternalLink = useCallback((url: string) => {
    if (!desktopBridge.isDesktop()) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }

    // 桌面端：走 IPC 让主进程打开系统浏览器
    // fire-and-forget：主进程立即返回，不等浏览器打开，
    // 避免渲染进程后续处理抢回焦点导致浏览器窗口层级变低
    void desktopBridge.openTargetPage({ mode: "browser", url }).catch(() => {
      toast.error(t("chat.markdownLink.openExternalFailed"));
    });
  }, [desktopBridge, toast, t]);

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
  const selectedMainAgentModel = selectedModel?.source === "internal-agent" ? selectedModel : null;
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
    ? t("chat.management.modelEmpty.noTextModel")
    : t("chat.management.modelEmpty.noneForAgent", { name: selectedAgent?.name ?? effectiveAgentCode });
  const currentUserDisplayName = authSession?.user.displayName ?? "";
  const { createInspectionAttachmentUpload } = useInspection();

  useEffect(() => {
    let active = true;
    const sessionCode = activeSession?.sessionCode ?? "";
    if (!sessionCode) {
      setUserInputRequests([]);
      setUserInputRequest(null);
      return () => {
        active = false;
      };
    }
    void listChatUserInputRequests(sessionCode)
      .then((requests) => {
        if (!active || activeSessionCodeRef.current !== sessionCode) return;
        setUserInputRequests(requests);
        setUserInputRequest(
          requests.find((request) => request.status === "Pending") ?? null,
        );
      })
      .catch(() => {
        if (active && activeSessionCodeRef.current === sessionCode) {
          setUserInputRequests([]);
          setUserInputRequest(null);
        }
      });
    return () => {
      active = false;
    };
  }, [activeChatDataRefreshKey, activeSession?.sessionCode]);

  function setSendingState(value: boolean) {
    sendingRef.current = value;
    setSending(value);
  }

  function scheduleSessionTitleRefresh(session: ChatSession) {
    const defaultTitle = session.title.trim().toLocaleLowerCase();
    if (defaultTitle !== "new chat" && defaultTitle !== "main agent") return;
    let attempts = 0;
    const refreshUntilTitleChanges = () => {
      attempts += 1;
      setActiveChatDataRefreshKey((current) => current + 1);
      window.dispatchEvent(new Event("tron:chat-session-updated"));

      const refreshedSession = findSession(
        sessionGroupsRef.current,
        session.sessionCode,
      );
      if (refreshedSession && refreshedSession.title !== session.title) return;
      if (attempts >= 35) return;

      const timer = window.setTimeout(() => {
        sessionTitleRefreshTimersRef.current = sessionTitleRefreshTimersRef.current.filter(
          (candidate) => candidate !== timer,
        );
        refreshUntilTitleChanges();
      }, 1000);
      sessionTitleRefreshTimersRef.current.push(timer);
    };
    refreshUntilTitleChanges();
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
    if (!selectedModel || !selectedModel.enabled) return null;
    const input: SendChatMessageInput = {
      agentCode: effectiveAgentCode,
      appCode: session.appCode,
      clientCapabilities: { userInputRequests: true },
      content,
      taskCode: session.taskCode,
      workspaceCode: session.workspaceCode,
    };
    if (attachments.length) input.attachments = attachments;
    if (selectedModel.source === "system-agent") {
      input.systemModelId = selectedModel.modelName;
    } else {
      // The selector always sends the complete selection. In particular Auto
      // intentionally does not include a physical modelCode.
      input.modelSelection = selectedModel.modelSelection;
      input.reasoningEffort = selectedReasoningEffort;
    }
    return input;
  }

  function toPageChatSession(session: RuntimeChatSession): ChatSession {
    return {
      ...session,
      sourceType: session.sourceType ?? undefined,
    };
  }

  const createRuntimeSendInput = useCallback(
    (session: RuntimeChatSession, content: string, attachments: ChatMessageAttachment[]) => (
      createSendInput(toPageChatSession(session), content, attachments)
    ),
    [effectiveAgentCode, selectedModel, selectedReasoningEffort],
  );

  const handleRuntimeSessionChange = useCallback((nextSession: RuntimeChatSession | null) => {
    if (nextSession) {
      setSessionGroups((current) => replaceSession(current, toPageChatSession(nextSession)));
      window.dispatchEvent(new Event("tron:chat-session-updated"));
    }
    setActiveChatDataRefreshKey((current) => current + 1);
    setActiveChatReplayRefreshKey((current) => current + 1);
    setActiveRunRefreshKey((current) => current + 1);
  }, []);

  const handleRuntimeRunStateChange = useCallback((state: ChatRuntimeRunState) => {
    if (!state.sessionCode) return;
    setSending(state.active);
    setRunningSessionCode((current) => {
      if (state.active) return state.sessionCode;
      return current === state.sessionCode ? "" : current;
    });
    setActiveRunCode(state.active ? state.runCode : "");
    setActiveRunSupportsSteer(state.active ? state.supportsSteer : null);
    setActiveRunRefreshKey((current) => current + 1);
  }, []);

  async function uploadAttachmentForActiveSession(
    file: File,
    options: Omit<Parameters<typeof uploadChatSessionAttachment>[1], "file"> = {},
  ): Promise<ChatMessageAttachment | null> {
    const targetSession = activeSession;
    if (!targetSession) {
      toast.warning(t("chat.management.toasts.attachNoSession"), { title: t("chat.management.toasts.sessionRequiredTitle") });
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
      toast.error(error instanceof Error ? error.message : t("chat.management.toasts.attachmentUploadFailed"), {
        title: t("chat.management.toasts.attachmentUploadFailed"),
      });
      return null;
    } finally {
      setAttachmentUploadPending(false);
    }
  }

  async function handleAttachmentFile(file: File) {
    const attachment = await uploadAttachmentForActiveSession(file, {
      kind: file.type.startsWith("image/") ? "image" : "file",
      summary: file.name,
    });
    if (!attachment) return;
    setPendingAttachments((current) => [...current, attachment]);
  }

  async function handleAttachmentFileInputChange(
    event: ChangeEvent<HTMLInputElement>,
  ) {
    const file = event.currentTarget.files?.item(0);
    event.currentTarget.value = "";
    if (!file) return;
    await handleAttachmentFile(file);
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
        toast.warning(t("chat.management.toasts.selectModelFirst"), { title: t("chat.management.toasts.modelRequiredTitle") });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.management.toasts.annotationUploadFailed"), {
        title: t("chat.management.toasts.attachmentUploadFailed"),
      });
    }
  }

  const handleInspectionConfirmForRightPanel = useStableCallback(handleInspectionConfirm);

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
    const activeSession = requestedSessionCode
      ? findSession(sessionGroups, requestedSessionCode)
      : undefined;
    if (!activeSession?.title.trim()) return;
    const tab = useTabStore
      .getState()
      .tabs.find((item) => item.activeNav === `chat-session:${activeSession.sessionCode}`);
    if (tab && tab.title !== activeSession.title) {
      useTabStore.getState().updateTab(tab.id, {
        title: activeSession.title,
        titleTranslationKey: undefined,
      });
    }
  }, [requestedSessionCode, sessionGroups]);

  useEffect(() => {
    const refresh = () => setActiveChatDataRefreshKey((current) => current + 1);
    window.addEventListener("tron:chat-session-archived", refresh);
    return () => window.removeEventListener("tron:chat-session-archived", refresh);
  }, []);

  useEffect(() => () => {
    window.dispatchEvent(new CustomEvent("tron:chat-session-run-state", {
      detail: { sessionCode: "" },
    }));
  }, []);

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
        const options = getServiceDataSource("model") === "api"
          ? await fetchWorkspaceSelectOptions(activeWorkspaceCode)
          : getWorkspaceSelectOptions(activeWorkspaceCode);
        if (!active) return;
        const models = availableChatModels(options);
        setChatModels(models);
      } catch (error) {
        if (active) {
          setChatModels([]);
          setSelectedModelCode("");
          toast.error(error instanceof Error ? error.message : t("chat.management.toasts.modelLoadFailed"), { title: t("chat.management.toasts.modelLoadFailedTitle") });
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
        const [internalRecords, systemRecords] = getServiceDataSource("agent") === "api"
          ? await Promise.all([
            fetchWorkspaceInternalAgents(activeWorkspaceCode),
            fetchWorkspaceSystemAgents(activeWorkspaceCode),
          ])
          : [
            listWorkspaceInternalAgentsMock(activeWorkspaceCode),
            listWorkspaceSystemAgentsMock(activeWorkspaceCode),
          ];
        if (!active) return;
        const agents = availableChatAgents(internalRecords, systemRecords);
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
          toast.error(error instanceof Error ? error.message : t("chat.management.toasts.agentLoadFailed"), { title: t("chat.management.toasts.agentLoadFailedTitle") });
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
    const sessionAgentCode = (activeSession?.agentCode ?? selectedAgentCode ?? mainChatAgentOption.agentCode).toLowerCase();
    const selectionKey = sessionCode ? sessionModelSelectionKey(sessionCode, sessionAgentCode) : "";
    const switchedSession = modelSelectionSessionRef.current !== sessionCode;

    if (switchedSession) {
      modelSelectionSessionRef.current = sessionCode;
      const persistedOptionCode = selectionKey ? sessionModelSelections[selectionKey] : "";
      const defaultModel = defaultChatModelOption(agentScopedModels);
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
      const persistedOptionCode = selectionKey ? sessionModelSelections[selectionKey] : "";
      if (persistedOptionCode && agentScopedModels.some((model) => model.optionCode === persistedOptionCode)) {
        return persistedOptionCode;
      }
      return defaultChatModelOption(agentScopedModels)?.optionCode ?? "";
    });
  }, [activeSession?.agentCode, activeSession?.sessionCode, agentScopedModels, chatModels, selectedAgentCode, sessionModelSelections]);

  useEffect(() => {
    const sessionCode = activeSession?.sessionCode;
    if (!sessionCode || !selectedModelCode) return;
    const activeSessionAgentCode = activeSession.agentCode ?? mainChatAgentOption.agentCode;
    if (selectedAgentCode !== activeSessionAgentCode) return;
    if (!agentScopedModels.some((model) => model.optionCode === selectedModelCode)) return;
    const selectionKey = sessionModelSelectionKey(sessionCode, activeSessionAgentCode);
    setSessionModelSelections((current) => (
      current[selectionKey] === selectedModelCode
        ? current
        : {
          ...current,
          [selectionKey]: selectedModelCode,
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
  }, [activeSession?.sessionCode, selectedModel?.optionCode, selectedMainAgentModel?.optionCode]);

  useEffect(() => {
    if (requestedSessionCode || requestedSourceTab || activeSessionCode) return;
    const defaultSession = firstSessionForSourceTab(sessionGroups, defaultChatSessionSourceTab);
    if (defaultSession) {
      setActiveSessionCode(defaultSession.sessionCode);
      selectSourceTab(defaultChatSessionSourceTab);
      replaceChatSearchParams((next) => {
        next.set("sessionCode", defaultSession.sessionCode);
      });
    }
  }, [
    activeSessionCode,
    replaceChatSearchParams,
    requestedSessionCode,
    requestedSourceTab,
    sessionGroups,
  ]);

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
  const liveAguiPendingAnchorMessageCode = activeSession?.sessionCode
    ? liveAguiPendingAnchorMessageCodesBySessionCode[activeSession.sessionCode] ?? ""
    : "";
  const contextAguiEvents = useMemo<TranscriptAguiEvent[]>(() => (
    mergeTranscriptAguiEvents<TranscriptAguiEvent>(
      aguiEvents,
      aguiProtocolEvents,
      liveAguiEvents,
    )
  ), [aguiEvents, aguiProtocolEvents, liveAguiEvents]);
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
    if (liveAguiPendingAnchorMessageCode && visibleMessages.some((message) => (
      message.sessionCode === activeSession.sessionCode
      && message.messageCode === liveAguiPendingAnchorMessageCode
    ))) {
      return liveAguiPendingAnchorMessageCode;
    }
    for (let index = visibleMessages.length - 1; index >= 0; index -= 1) {
      const message = visibleMessages[index];
      if (message.sessionCode === activeSession.sessionCode && message.role === "USER") {
        return message.messageCode;
      }
    }
    return "";
  }, [activeSession?.sessionCode, liveAguiPending, liveAguiPendingAnchorMessageCode, visibleMessages]);
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
  const sidebarRunningSessionCode = sending
    ? runningSessionCode
    : currentSessionRunActive
      ? activeSession?.sessionCode ?? ""
      : "";
  useEffect(() => {
    window.dispatchEvent(new CustomEvent("tron:chat-session-run-state", {
      detail: { sessionCode: sidebarRunningSessionCode },
    }));
  }, [sidebarRunningSessionCode]);
  const effectiveActiveRunCode = runUiState.runCode;
  const canSteerCurrentRun = runUiState.canSteer;
  const canStopCurrentRun = runUiState.canStop;
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
    || userInputRequest
    || (!composer.trim() && !pendingAttachments.length)
    || creatingSession,
  );
  const steerDisabled = queueDisabled || steering || !canSteerCurrentRun;
  const hasOpenReplayRun = Boolean(replayActiveRunHint);
  const activeRunConfirmedIdle = Boolean(
    activeRunQueryMatchesSession
    && !activeRunState.loading
    && !activeRunState.error
    && !authoritativeActiveRun
  );
  const shouldPollActiveChatData = Boolean(
    activeSession?.sessionCode
    && !(sending && runningSessionCode === activeSession.sessionCode)
    && shouldPollChatRecoveryData({
      activeRunConfirmedIdle,
      hasActiveRun: currentSessionRunActive,
      hasOpenReplayRun,
      hasPendingUserInput: Boolean(userInputRequest),
      hasQueuedSubmission: hasQueuedSubmissionForActiveSession,
    }),
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

  const activeRightPanelSnapshot = useMemo<ChatRightPanelSnapshot | null>(() => {
    if (!activeSession) return null;

    return {
      compactionBusy: compactingSessionCode === activeSession.sessionCode,
      contextEvents: contextAguiEvents,
      goal: activeGoal,
      goalBusy: goalActionSessionCode === activeSession.sessionCode,
      goalStatus,
      latestCompaction,
      latestCompactionStatus,
      latestRequestEstimate: latestRequestEstimatesBySessionCode[activeSession.sessionCode] ?? null,
      latestUsageAudit: latestUsageAuditsBySessionCode[activeSession.sessionCode] ?? null,
      messages: visibleMessages,
      model: selectedMainAgentModel,
      selectedModel,
      session: activeSession,
      todos: shortTermMemoryTodos,
      todosStatus: shortTermMemoryTodosStatus,
      workspaceGoalRunning,
    };
  }, [
    activeSession,
    activeGoal,
    compactingSessionCode,
    contextAguiEvents,
    goalActionSessionCode,
    goalStatus,
    latestCompaction,
    latestCompactionStatus,
    latestRequestEstimatesBySessionCode,
    latestUsageAuditsBySessionCode,
    selectedMainAgentModel,
    selectedModel,
    shortTermMemoryTodos,
    shortTermMemoryTodosStatus,
    visibleMessages,
    workspaceGoalRunning,
  ]);

  useEffect(() => {
    if (!activeRightPanelSnapshot) return;
    const sessionCode = activeRightPanelSnapshot.session.sessionCode;
    setRightPanelSnapshots((current) => (
      current[sessionCode] === activeRightPanelSnapshot
        ? current
        : { ...current, [sessionCode]: activeRightPanelSnapshot }
    ));
  }, [activeRightPanelSnapshot]);

  const rightPanel = useMemo(() => {
    if (!hasActiveSession) return undefined;
    return (
      <ChatSessionRightPanelCache
        activeSessionCode={activeSession?.sessionCode ?? ""}
        snapshots={rightPanelSnapshots}
        onCancelGoal={handleCancelGoalForRightPanel}
        onClearGoal={handleClearGoalForRightPanel}
        onCreateCompaction={handleCreateCompactionForRightPanel}
        onCreateGoal={handleCreateGoalForRightPanel}
        onPauseGoal={handlePauseGoalForRightPanel}
        onResumeGoal={handleResumeGoalForRightPanel}
        onUpdateGoal={handleUpdateGoalForRightPanel}
      />
    );
  }, [
    activeSession?.sessionCode,
    handleCancelGoalForRightPanel,
    handleClearGoalForRightPanel,
    handleCreateCompactionForRightPanel,
    handleCreateGoalForRightPanel,
    handlePauseGoalForRightPanel,
    handleResumeGoalForRightPanel,
    handleUpdateGoalForRightPanel,
    hasActiveSession,
    rightPanelSnapshots,
  ]);
  const downloadPendingAttachment = useCallback(
    (attachment: ChatMessageAttachment) => {
      if (!activeSession) return Promise.reject(new Error("No active chat session"));
      return downloadChatSessionAttachment(activeSession, attachment).then((response) => response.blob);
    },
    [activeSession],
  );
  const transcriptWorkReplayOptions = useMemo(() => ({
    recentTurnLimit: aguiWorkReplayPreferences.recentTurnLimit,
  }), [aguiWorkReplayPreferences.recentTurnLimit]);
  const transcriptAttachmentHeader = pendingAttachments.length ? (
    <div className="flex max-w-full flex-wrap gap-[12px]">
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
    <div className="flex w-full min-w-0 max-w-full flex-col items-start gap-[12px]">
      <div className="flex w-full min-w-0 max-w-full flex-wrap items-center gap-1">
        <input
          className="hidden"
          ref={attachmentFileInputRef}
          type="file"
          onChange={(event) => { void handleAttachmentFileInputChange(event); }}
        />
        <Button
          aria-label={t("chat.management.composer.uploadAttachment")}
          disabled={!activeSession || attachmentUploadPending}
          size="icon"
          title={attachmentUploadPending ? t("chat.management.composer.uploading") : t("chat.management.composer.uploadAttachment")}
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
              className="inline-flex h-8 flex-none items-center gap-[12px] rounded-md border-0 bg-transparent px-2 text-xs text-[var(--color-on-surface-variant)] shadow-none transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
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
  const attachmentLabels = useMemo<ChatAttachmentLabels>(() => ({
    annotationPreview: t("chat.attachments.annotationPreview"),
    attachments: t("chat.attachments.attachments"),
  }), [i18n.language, t]);

  useTabPanelConfig({
    onPageInspectionConfirm: handleInspectionConfirmForRightPanel,
    rightPanel,
  });

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
    const selectionKey = sessionModelSelectionKey(sessionCode, nextAgentCode);
    setSessionModelSelections((current) => (
      current[selectionKey] === optionCode
        ? current
        : {
          ...current,
          [selectionKey]: optionCode,
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
      toast.info(t("chat.management.toasts.chatStopped"), { title: t("chat.management.toasts.chatStoppedTitle") });
    } catch (error) {
      if (!isAbortError(error)) {
        toast.error(
          error instanceof Error ? error.message : t("chat.management.toasts.chatStopFailed"),
          { title: t("chat.management.toasts.chatStopFailedTitle") },
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

  function forgetRightPanelSnapshot(sessionCode: string) {
    setRightPanelSnapshots((current) => {
      if (!(sessionCode in current)) return current;
      const next = { ...current };
      delete next[sessionCode];
      return next;
    });
  }

  function forgetComposerDraft(sessionCode: string) {
    setComposerDraftsBySessionCode((current) => {
      if (!(sessionCode in current)) return current;
      const next = { ...current };
      delete next[sessionCode];
      return next;
    });
    setPendingAttachmentsBySessionCode((current) => {
      if (!(sessionCode in current)) return current;
      const next = { ...current };
      delete next[sessionCode];
      return next;
    });
  }

  async function handleSend(contentOverride?: string, attachmentOverride?: ChatMessageAttachment[]) {
    if (sendingRef.current || userInputRequest || userInputAnswerPending) return;
    if (!activeSession) return;
    const rawContent = contentOverride ?? composer;
    const goalCommand = contentOverride === undefined
      ? parseGoalCommand(rawContent)
      : { isGoalCommand: false, content: rawContent.trim() };
    const content = goalCommand.content;
    const attachments = attachmentOverride ?? pendingAttachments;
    if (!content && !attachments.length) {
      toast.warning(
        goalCommand.isGoalCommand ? t("chat.management.toasts.goalRequired") : t("chat.management.toasts.nothingToSend"),
        { title: goalCommand.isGoalCommand ? t("chat.management.toasts.goalRequiredTitle") : t("chat.management.toasts.nothingToSendTitle") },
      );
      return;
    }
    if (!selectedModel) {
      toast.warning(t("chat.management.toasts.selectModelFirst"), { title: t("chat.management.toasts.modelRequiredTitle") });
      return;
    }
    if (!selectedModel.enabled) {
      toast.warning(
        selectedModel.remoteStatus === "Unavailable"
          ? "该远程模型已经下线，请改选其他模型。"
          : "该模型当前不可用，请改选其他模型。",
        { title: t("chat.management.toasts.modelRequiredTitle") },
      );
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
      setComposerDraftsBySessionCode((current) => ({
        ...current,
        [sendingSessionCode]: "",
      }));
      setPendingAttachmentsBySessionCode((current) => ({
        ...current,
        [sendingSessionCode]: [],
      }));
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
      setUserInputRequest(
        result.userInputRequest?.status === "Pending"
          ? result.userInputRequest
          : null,
      );
      const pendingUserInputRequest = result.userInputRequest;
      if (pendingUserInputRequest?.status === "Pending") {
        setUserInputRequests((current) => [
          ...current.filter((request) => request.requestCode !== pendingUserInputRequest.requestCode),
          pendingUserInputRequest,
        ]);
      }
      setSessionGroups((current) => replaceSession(current, result.session));
      window.dispatchEvent(new Event("tron:chat-session-updated"));
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
          t("chat.management.toasts.responseRecovered"),
          { title: t("chat.management.toasts.responseRecoveredTitle") },
        );
      } else {
        const rawMessage = error instanceof Error ? error.message : t("chat.management.toasts.chatSendFailed");
        const toastMessage = chatSendErrorToastMessage(rawMessage);
        if (!sourceSubmission && pendingSubmissionAttachments.length) {
          setPendingAttachmentsBySessionCode((current) => ({
            ...current,
            [sendingSessionCode]: [
              ...pendingSubmissionAttachments,
              ...(current[sendingSessionCode] ?? []),
            ],
          }));
        }
        if (activeSessionCodeRef.current === sendingSessionCode) {
          setVisibleMessages((current) => current.filter((item) => (
            item.messageCode !== optimisticAssistantCode || Boolean(item.content.trim())
          )));
        }
        toast.error(toastMessage, { title: t("chat.management.toasts.chatSendFailedTitle") });
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
    if (queueDisabled || userInputRequest || userInputAnswerPending || !activeSession) return;
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

  async function handleAnswerUserInput(answers: ChatUserInputAnswer[]) {
    const targetSession = activeSession;
    const request = userInputRequest;
    if (!targetSession || !request || userInputAnswerPending) return;
    const continuationAnchorMessageCode = [...visibleMessages]
      .reverse()
      .find((message) => message.role === "ASSISTANT")?.messageCode ?? "";
    setUserInputAnswerPending(true);
    setSendingState(true);
    runningSessionCodeRef.current = targetSession.sessionCode;
    setRunningSessionCode(targetSession.sessionCode);
    activeRunCodeRef.current = "";
    activeRunSupportsSteerRef.current = null;
    setActiveRunCode("");
    setActiveRunSupportsSteer(null);
    setLiveAguiPendingAnchorMessageCodesBySessionCode((current) => ({
      ...current,
      [targetSession.sessionCode]: continuationAnchorMessageCode,
    }));
    setLiveAguiEventsBySessionCode((current) => ({
      ...current,
      [targetSession.sessionCode]: [],
    }));
    setLiveAguiAnchorMessageCodesBySessionCode((current) => ({
      ...current,
      [targetSession.sessionCode]: {},
    }));
    try {
      const result = await streamChatUserInputAnswer(
        targetSession,
        request.requestCode,
        { answers },
        {
          onAguiEvent: (event) => {
            if (activeSessionCodeRef.current !== targetSession.sessionCode) return;
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
              setLiveAguiAnchorMessageCodesBySessionCode((current) => {
                const sessionAnchors = current[targetSession.sessionCode] ?? {};
                if (sessionAnchors[runCode]) return current;
                return {
                  ...current,
                  [targetSession.sessionCode]: {
                    ...sessionAnchors,
                    [runCode]: continuationAnchorMessageCode,
                  },
                };
              });
            }
            setLiveAguiEventsBySessionCode((current) => ({
              ...current,
              [targetSession.sessionCode]: mergeTranscriptAguiEvents(
                current[targetSession.sessionCode] ?? [],
                [event],
              ),
            }));
          },
        },
      );
      if (activeSessionCodeRef.current === targetSession.sessionCode) {
        setVisibleMessages((current) =>
          mergeChatMessagesChronologically(current, result.messages),
        );
        const nextRequests = await listChatUserInputRequests(targetSession.sessionCode).catch(() => null);
        if (nextRequests) {
          setUserInputRequests(nextRequests);
          setUserInputRequest(
            nextRequests.find((item) => item.status === "Pending") ?? null,
          );
        } else {
          setUserInputRequest(
            result.userInputRequest?.status === "Pending"
              ? result.userInputRequest
              : null,
          );
          setUserInputRequests((current) => current.map((item) => (
            item.requestCode === request.requestCode
              ? { ...item, answers, status: "Answered" as const }
              : item
          )));
        }
      }
      setSessionGroups((current) => replaceSession(current, result.session));
      window.dispatchEvent(new Event("tron:chat-session-updated"));
      setActiveChatDataRefreshKey((current) => current + 1);
      setActiveRunRefreshKey((current) => current + 1);
      setActiveChatReplayRefreshKey((current) => current + 1);
      setShortTermMemoryRefreshKey((current) => current + 1);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.management.toasts.answerFailed"), {
        title: t("chat.management.toasts.answerFailedTitle"),
      });
    } finally {
      setLiveAguiPendingAnchorMessageCodesBySessionCode((current) => {
        if (!(targetSession.sessionCode in current)) return current;
        const next = { ...current };
        delete next[targetSession.sessionCode];
        return next;
      });
      clearLocalStreamState();
      setUserInputAnswerPending(false);
    }
  }

  async function handleSteerMessage() {
    if (steeringRef.current || steerDisabled || userInputRequest || userInputAnswerPending || !activeSession) return;
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
        toast.info(t("chat.management.toasts.steerDiscarded"), { title: t("chat.management.toasts.sessionClosedTitle") });
      } else if (safeQueueFallback) {
        toast.info(
          t("chat.management.toasts.steerQueued"),
          { title: t("chat.management.toasts.messageQueuedTitle") },
        );
      } else if (isAbortError(error)) {
        toast.info(
          t("chat.management.toasts.steerNotRetried"),
          { title: t("chat.management.toasts.steerNotRetriedTitle") },
        );
      } else {
        toast.error(
          error instanceof Error ? error.message : t("chat.management.toasts.steerFailed"),
          { title: t("chat.management.toasts.steerFailedTitle") },
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
      toast.warning(t("chat.management.toasts.selectModelFirst"), { title: t("chat.management.toasts.modelRequiredTitle") });
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
      toast.warning(t("chat.management.toasts.selectModelFirst"), { title: t("chat.management.toasts.modelRequiredTitle") });
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
      toast.warning(t("chat.management.toasts.createWorkspaceFirst"), { title: t("chat.management.toasts.noWorkspaceTitle") });
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
    } catch (error) {
      const message = error instanceof Error ? error.message : t("chat.management.toasts.createSessionFailed");
      toast.error(message, { title: t("chat.management.toasts.createSessionFailed") });
    } finally {
      setCreatingSession(false);
    }
  }

  useEffect(() => {
    const handleShortcut = () => {
      void handleCreateSession();
    };

    window.addEventListener(CREATE_CHAT_SESSION_SHORTCUT_EVENT, handleShortcut);
    return () => window.removeEventListener(
      CREATE_CHAT_SESSION_SHORTCUT_EVENT,
      handleShortcut,
    );
  }, [handleCreateSession]);

  async function handleArchiveSession(session: ChatSession) {
    const returnSourceTab = requestedSourceTab ?? activeSourceTab;
    const shouldCreateProjectReplacement = Boolean(
      session.projectCode?.trim()
      && activeSessionCode === session.sessionCode,
    );
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
      forgetSessionModelSelection(archived.sessionCode);
      forgetRightPanelSnapshot(archived.sessionCode);
      forgetComposerDraft(archived.sessionCode);

      if (shouldCreateProjectReplacement) {
        try {
          setCreatingSession(true);
          const replacement = await createChatSession({
            agentCode: mainChatAgentOption.agentCode,
            projectCode: session.projectCode,
            title: "New chat",
            workspaceCode: session.workspaceCode,
          });
          invalidatedQueueSessionCodesRef.current.delete(replacement.sessionCode);
          setSessionGroups((current) => insertSession(current, replacement));
          handleSessionSelect(replacement, returnSourceTab);
          setVisibleMessages([]);
        } catch (error) {
          const message = error instanceof Error ? error.message : t("chat.management.toasts.createSessionFailed");
          toast.error(message, { title: t("chat.management.toasts.createSessionFailed") });
          selectSessionAfterRemoval(nextGroups, returnSourceTab);
        } finally {
          setCreatingSession(false);
        }
      } else if (activeSessionCode === archived.sessionCode) {
        selectSessionAfterRemoval(nextGroups, returnSourceTab);
      }
      toast.success(t("chat.management.toasts.sessionArchived"), { title: archived.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : t("chat.management.toasts.archiveSessionFailed");
      toast.error(message, { title: t("chat.management.toasts.archiveSessionFailed") });
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
      forgetRightPanelSnapshot(deleted.sessionCode);
      forgetComposerDraft(deleted.sessionCode);
      // Clear MainAgentChatWindow's persisted session if it matches the deleted one
      forgetMainAgentPersistedSession(activeWorkspaceCode, deleted.sessionCode);
      toast.success(t("chat.management.toasts.sessionDeleted"), { title: deleted.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : t("chat.management.toasts.deleteSessionFailed");
      toast.error(message, { title: t("chat.management.toasts.deleteSessionFailed") });
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
      toast.warning(t("chat.management.toasts.titleRequired"), { title: t("chat.management.toasts.renameSkippedTitle") });
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
      window.dispatchEvent(new Event("tron:chat-session-updated"));
      setEditingTitle(false);
      toast.success(t("chat.management.toasts.sessionRenamed"), { title: session.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : t("chat.management.toasts.renameSessionFailed");
      toast.error(message, { title: t("chat.management.toasts.renameSessionFailed") });
    } finally {
      setRenamingSession(false);
    }
  }

  async function handleSidebarRenameSession(session: ChatSession, title: string) {
    if (!title.trim()) {
      toast.warning(t("chat.management.toasts.titleRequired"), { title: t("chat.management.toasts.renameSkippedTitle") });
      return;
    }

    setSessionActionPendingCode(session.sessionCode);
    try {
      const renamed = await updateChatSession(session.sessionCode, { title });
      setSessionGroups((current) => replaceSession(current, renamed));
      window.dispatchEvent(new Event("tron:chat-session-updated"));
      toast.success(t("chat.management.toasts.sessionRenamed"), { title: renamed.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : t("chat.management.toasts.renameSessionFailed");
      toast.error(message, { title: t("chat.management.toasts.renameSessionFailed") });
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
      <div className="hidden">
        <SessionSidebar
          activeSessionCode={activeSession?.sessionCode ?? activeSessionCode}
          activeSourceTab={activeSourceTab}
          collapsed={sidebarCollapsed}
          creatingSession={creatingSession}
          groups={sessionGroups}
          pendingUserInputSessionCodes={pendingUserInputSessionCodes}
          runningSessionCode={currentSessionRunActive ? activeSession?.sessionCode ?? "" : ""}
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
      </div>
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
                title={activeSession ? t("chat.management.header.doubleClickRename") : undefined}
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
                {activeSession?.title ?? t("chat.management.header.defaultTitle")}
              </h1>
            )}
            {hasActiveSession ? (
              <p className="mt-0.5 flex items-center gap-[12px] text-xs text-[var(--color-on-surface-variant)]">
                <CircleDot size={12} />
                {activeSession?.sessionCode}
                {currentSessionRunActive ? (
                  <span className="inline-flex items-center gap-1 text-[var(--color-primary)]" role="status">
                    <LoaderCircle className="size-3 animate-spin" />
                    {t("chat.management.runIndicator.running")}
                  </span>
                ) : null}
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

        <TronChatRuntimeWindow
          adapter={chatRuntimeAdapter}
          attachmentLabels={attachmentLabels}
          autoFocusComposer={shouldAutoFocusChatComposer()}
          composerRef={composerRef}
          composerHeader={transcriptAttachmentHeader}
          config={{
            agentCode: effectiveAgentCode,
            disabledInput: creatingSession || !activeSession || !selectedModel,
            floating: false,
            inputPlaceholder: t("chat.management.composer.inputPlaceholder"),
            showAgui: true,
            storageKey: "tron.chat.management.runtime",
            title: activeSession?.title ?? t("chat.management.header.defaultTitle"),
            userInputLabels,
            workRailLabels,
            workspaceCode: activeWorkspaceCode,
          }}
          createSendInput={createRuntimeSendInput}
          createSessionLabel={t("chat.management.emptyState.startChat")}
          currentUserDisplayName={currentUserDisplayName}
          debugOptions={transcriptWorkReplayOptions}
          emptyDescription={activeWorkspaceCode
            ? t("chat.management.emptyState.withWorkspace")
            : t("chat.management.emptyState.noWorkspace")}
          emptyTitle={t("chat.management.emptyState.noSessionTitle")}
          formatAnnotationCount={formatAnnotationCount}
          draft={composer}
          initialSession={activeSession ?? null}
          onCreateSession={() => {
            void handleCreateSession();
          }}
          onDraftChange={setComposer}
          onPendingAttachmentsChange={setPendingAttachments}
          onRunStateChange={handleRuntimeRunStateChange}
          onSessionChange={handleRuntimeSessionChange}
          pendingAttachments={pendingAttachments}
          showAttachmentControl={false}
          showHeader={false}
          footerAddon={transcriptFooterAddon}
          renderMarkdownLink={({ href, children }) => (
            <MarkdownLink
              href={href}
              onOpenInternal={openMarkdownInternalLink}
              onOpenExternal={openMarkdownExternalLink}
            >
              {children}
            </MarkdownLink>
          )}
        />
      </main>
      <ConfirmDialog
        ariaLabel={t("chat.management.deleteDialog.aria")}
        cancelText={t("chat.management.deleteDialog.cancel")}
        confirmText={sessionActionPendingCode === deleteTargetSession?.sessionCode ? t("chat.management.deleteDialog.confirming") : t("chat.management.deleteDialog.confirm")}
        description={deleteTargetSession
          ? t("chat.management.deleteDialog.description", { title: deleteTargetSession.title })
          : undefined}
        open={Boolean(deleteTargetSession)}
        pending={sessionActionPendingCode === deleteTargetSession?.sessionCode}
        title={t("chat.management.deleteDialog.title")}
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
