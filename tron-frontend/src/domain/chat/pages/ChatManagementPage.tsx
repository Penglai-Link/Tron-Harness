import {
  Fragment,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Bot,
  BrainCircuit,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  CircleDot,
  Ellipsis,
  GitBranch,
  Hammer,
  LoaderCircle,
  MessageSquareText,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Send,
  Terminal,
  UserRound,
  Wrench,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
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
} from "@/domain/chat/model";

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
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

function contextWindowLimit(model: ModelRecord | null | undefined) {
  return model?.contextWindowTokens ?? null;
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

type TranscriptAguiEvent = ChatAguiEvent | ChatAguiStreamEvent;

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

function availableChatModels(models: ModelRecord[]) {
  return models.filter((model) => model.enabled && model.modelType === "TEXT");
}

function defaultChatModel(models: ModelRecord[]) {
  return models.find((model) => model.isDefault) ?? models[0] ?? null;
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
  session,
}: {
  events: ChatAguiEvent[];
  messages: ChatMessage[];
  model?: ModelRecord | null;
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
    ?? contextValue(latestEventValue(events, "modelName"))
    ?? "unavailable"
  );
  const usage = chatTokenUsage(messages);
  const usedTokens = usage.total;
  const windowTokens = contextWindowLimit(model);

  return (
    <section className="space-y-4">
      <h2 className="text-xl font-semibold">Run Context</h2>
      <div className="space-y-5 text-sm">
        <ContextBlock title="Context Window">
          <ContextRow label="model" value={modelLabel} />
          <ContextRow label="window" value={formatContextWindowValue(windowTokens, usedTokens)} />
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

function ContextRow({ icon, label, value }: { icon?: React.ReactNode; label: string; value: string }) {
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
  loading,
  models,
  onSelect,
  selectedModelCode,
}: {
  loading: boolean;
  models: ModelRecord[];
  onSelect: (modelCode: string) => void;
  selectedModelCode: string;
}) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const selectedModel = models.find((model) => model.modelCode === selectedModelCode) ?? null;
  const disabled = loading || models.length === 0;

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
        className="inline-flex h-8 max-w-[340px] items-center gap-2 rounded-md px-2 text-xs text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:pointer-events-none disabled:opacity-50"
        type="button"
        aria-label="Select chat model"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => setOpen((value) => !value)}
      >
        <Bot size={14} />
        <span className="truncate">
          {loading ? "Loading models..." : selectedModel ? selectedModel.displayName : "No text model"}
        </span>
        <ChevronDown size={14} />
      </button>
      {open ? (
        <div
          ref={menuRef}
          className="absolute bottom-full left-0 z-30 mb-2 max-h-72 min-w-full w-max max-w-[240px] overflow-y-auto rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-1.5 text-sm shadow-[0_12px_28px_rgba(0,0,0,0.18)]"
          role="listbox"
          aria-label="Chat models"
        >
          {models.map((model) => (
            <button
              className={cn(
                "flex w-full items-center rounded-md px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]",
                model.modelCode === selectedModelCode && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
              )}
              key={model.modelCode}
              type="button"
              role="option"
              aria-selected={model.modelCode === selectedModelCode}
              onClick={() => {
                onSelect(model.modelCode);
                setOpen(false);
              }}
            >
              <span className="block min-w-0 truncate font-medium">{model.displayName}</span>
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

  if (collapsed) {
    const collapsedSessions = groups.flatMap((group) => group.sessions);

    return (
      <aside className="flex w-12 flex-col items-center border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] py-3">
        <button className={iconButtonClass()} type="button" aria-label="Expand sessions" onClick={onCollapseToggle}>
          <PanelLeftOpen size={17} />
        </button>
        <button
          className={cn(iconButtonClass(), "mt-2")}
          type="button"
          aria-label="New session"
          title="New session"
          disabled={creatingSession}
          onClick={onCreateSession}
        >
          <Plus size={17} />
        </button>
        <div className="mt-4 h-px w-6 bg-[var(--color-outline-variant)]" />
        <div className="mt-3 flex w-full flex-1 flex-col items-center gap-1 overflow-y-auto px-1">
          {collapsedSessions.map((session) => (
            <button
              className={iconButtonClass(activeSessionCode === session.sessionCode)}
              key={session.sessionCode}
              type="button"
              aria-label={`Open ${session.title}`}
              title={session.title}
              onClick={() => onSessionSelect(session.sessionCode)}
            >
              <MessageSquareText size={17} />
            </button>
          ))}
        </div>
      </aside>
    );
  }

  return (
    <aside className="w-[286px] shrink-0 overflow-hidden border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]">
      <div className="flex h-14 items-center justify-between border-b border-[var(--color-outline-variant)] px-4">
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
          <button className={iconButtonClass()} type="button" aria-label="Collapse sessions" onClick={onCollapseToggle}>
            <PanelLeftClose size={17} />
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
                            <span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">{formatTime(session.updatedAt)}</span>
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
  return (
    <article className={cn("max-w-[760px] rounded-md border bg-white px-4 py-3 shadow-sm", isUser ? "ml-auto border-[var(--color-primary)]" : "border-[var(--color-card-border)]")}>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          {isUser ? <UserRound size={16} /> : <Bot size={16} />}
          <span className="truncate text-sm font-semibold">{actorName}</span>
        </div>
        <span className="shrink-0 text-xs text-[var(--color-on-surface-variant)]">{formatTime(message.createdAt)}</span>
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

function AguiEventCard({ event }: { event: TranscriptAguiEvent }) {
  const type = aguiEventType(event);
  const summary = aguiEventSummary(event);

  return (
    <article className="rounded-md border border-[var(--color-outline-variant)] bg-white px-3 py-2 text-sm shadow-sm">
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

function AguiEventRail({
  events,
  preferences,
}: {
  events: TranscriptAguiEvent[];
  preferences: AguiDebugPreferences;
}) {
  const cardEvents = events.filter((event) => shouldShowAguiEventType(aguiEventType(event), preferences));
  const terminalEvent = terminalAguiEvent(events);
  const terminalRunCode = terminalEvent ? aguiEventRunCode(terminalEvent) : "";
  const [expanded, setExpanded] = useState(true);

  useEffect(() => {
    setExpanded(!terminalRunCode);
  }, [terminalRunCode]);

  if (!cardEvents.length) return null;

  return (
    <section className="max-w-[760px] space-y-2" aria-label="AG-UI event cards">
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
      {expanded ? cardEvents.map((event, index) => (
        <AguiEventCard
          key={`${aguiEventType(event)}-${aguiEventRunCode(event)}-${aguiEventTime(event)}-${index}`}
          event={event}
        />
      )) : null}
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
  const [liveAguiAnchorMessageCode, setLiveAguiAnchorMessageCode] = useState("");
  const [sending, setSending] = useState(false);
  const [creatingSession, setCreatingSession] = useState(false);
  const [chatModels, setChatModels] = useState<ModelRecord[]>([]);
  const [selectedModelCode, setSelectedModelCode] = useState("");
  const [modelsLoading, setModelsLoading] = useState(false);
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const [renamingSession, setRenamingSession] = useState(false);
  const [runningSessionCode, setRunningSessionCode] = useState("");
  const [sessionActionPendingCode, setSessionActionPendingCode] = useState("");
  const [unseenSessionCodes, setUnseenSessionCodes] = useState<Set<string>>(() => new Set());
  const activeSessionCodeRef = useRef("");
  const selectedModel = chatModels.find((model) => model.modelCode === selectedModelCode) ?? null;
  const currentUserDisplayName = authSession?.user.displayName ?? "";

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
        const defaultModel = defaultChatModel(models);
        setChatModels(models);
        setSelectedModelCode((current) => (
          current && models.some((model) => model.modelCode === current)
            ? current
            : defaultModel?.modelCode ?? ""
        ));
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
    if (!activeSessionCode && activeSession) {
      setActiveSessionCode(activeSession.sessionCode);
    }
  }, [activeSession, activeSessionCode]);

  useEffect(() => {
    activeSessionCodeRef.current = activeSessionCode;
  }, [activeSessionCode]);

  useEffect(() => {
    setVisibleMessages(messages);
    setLiveAguiEvents([]);
    setLiveAguiAnchorMessageCode("");
  }, [messages]);

  const transcriptAguiEvents = useMemo<TranscriptAguiEvent[]>(() => liveAguiEvents, [liveAguiEvents]);
  const liveAguiAnchorIndex = useMemo(() => {
    if (!transcriptAguiEvents.length) return -1;

    const anchoredIndex = visibleMessages.findIndex((message) => message.messageCode === liveAguiAnchorMessageCode);
    if (anchoredIndex >= 0) return anchoredIndex;

    for (let index = visibleMessages.length - 1; index >= 0; index -= 1) {
      if (visibleMessages[index].role === "USER") return index;
    }

    return -1;
  }, [liveAguiAnchorMessageCode, transcriptAguiEvents.length, visibleMessages]);

  const rightPanel = useMemo(() => (
    <RightContextSidebar events={aguiEvents} messages={visibleMessages} model={selectedModel} session={activeSession} />
  ), [activeSession, aguiEvents, selectedModel, visibleMessages]);

  useConsoleShellConfig({ rightPanel });

  function handleSessionSelect(sessionCode: string) {
    setActiveSessionCode(sessionCode);
    setUnseenSessionCodes((current) => {
      if (!current.has(sessionCode)) return current;
      const next = new Set(current);
      next.delete(sessionCode);
      return next;
    });
  }

  async function handleSend() {
    if (!activeSession) return;
    const content = composer.trim();
    if (!content) {
      toast.warning("Message content is required", { title: "Nothing to send" });
      return;
    }
    const sendingSessionCode = activeSession.sessionCode;
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
    setComposer("");
    setLiveAguiEvents([]);
    setLiveAguiAnchorMessageCode(optimisticUserCode);
    if (activeSessionCodeRef.current === sendingSessionCode) {
      setVisibleMessages((current) => [
        ...current,
        optimisticUserMessage,
        optimisticAssistantMessage,
      ]);
    }
    try {
      const result = await streamChatMessage(
        activeSession,
        {
          appCode: activeSession.appCode,
          content,
          modelCode: selectedModel?.modelCode ?? null,
          taskCode: activeSession.taskCode,
          workspaceCode: activeSession.workspaceCode,
        },
        {
          onAguiEvent: (event) => {
            if (event.threadId && event.threadId !== sendingSessionCode) return;
            setLiveAguiEvents((current) => [...current, event]);
          },
          onDelta: (delta) => {
            if (activeSessionCodeRef.current !== sendingSessionCode) return;
            setVisibleMessages((current) => current.map((message) => (
              message.messageCode === optimisticAssistantCode
                ? { ...message, content: `${message.content}${delta}` }
                : message
            )));
          },
        },
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
      const message = error instanceof Error ? error.message : "Chat send failed";
      if (activeSessionCodeRef.current === sendingSessionCode) {
        setVisibleMessages((current) => current.map((item) => (
          item.messageCode === optimisticAssistantCode
            ? { ...item, content: message }
            : item
        )));
      }
      toast.error(message, { title: "Chat send failed" });
    } finally {
      setSending(false);
      setRunningSessionCode("");
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
      toast.success("Session archived", { title: archived.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Archive session failed";
      toast.error(message, { title: "Archive session failed" });
    } finally {
      setSessionActionPendingCode("");
    }
  }

  async function handleDeleteSession(session: ChatSession) {
    if (!window.confirm(`Delete "${session.title}" permanently?`)) return;

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
      toast.success("Session deleted", { title: deleted.sessionCode });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Delete session failed";
      toast.error(message, { title: "Delete session failed" });
    } finally {
      setSessionActionPendingCode("");
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
    <div className="relative z-0 flex h-[calc(100vh-3.5rem)] min-h-[720px] min-w-0 overflow-hidden bg-white">
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
        onDeleteSession={(session) => void handleDeleteSession(session)}
        onRenameSession={handleSidebarRenameSession}
        onSessionSelect={handleSessionSelect}
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 items-center justify-between border-b border-[var(--color-outline-variant)] px-5">
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
            <p className="mt-0.5 flex items-center gap-2 text-xs text-[var(--color-on-surface-variant)]">
              <CircleDot size={12} />
              {activeSession?.sessionCode ?? "No session"}
              <span>·</span>
              {activeSession?.taskCode ? "task agent conversation" : "main agent conversation"}
            </p>
          </div>
        </header>

        <section className="flex-1 overflow-y-auto bg-[var(--color-surface-container-low)] px-5 py-5">
          <div className="mx-auto flex max-w-4xl flex-col gap-4">
            {visibleMessages.length ? visibleMessages.map((message) => (
              <Fragment key={message.messageCode}>
                <ChatBubble
                  currentUserDisplayName={currentUserDisplayName}
                  message={message}
                />
                {liveAguiAnchorIndex >= 0 && visibleMessages[liveAguiAnchorIndex]?.messageCode === message.messageCode ? (
                  <AguiEventRail events={transcriptAguiEvents} preferences={aguiDebugPreferences} />
                ) : null}
              </Fragment>
            )) : activeWorkspaceCode ? (
              <div className="flex min-h-[360px] items-center justify-center rounded-md border border-dashed border-[var(--color-outline-variant)] bg-white text-sm text-[var(--color-on-surface-variant)]">
                No persisted messages in this session.
              </div>
            ) : (
              <div className="flex min-h-[360px] items-center justify-center rounded-md border border-dashed border-[var(--color-outline-variant)] bg-white text-sm text-[var(--color-on-surface-variant)]">
                Select or create a workspace before loading chat sessions.
              </div>
            )}
          </div>
        </section>

        <footer className="border-t border-[var(--color-outline-variant)] bg-white px-4 py-3">
          <div className="flex min-h-28 w-full flex-col rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-3">
            <TextArea
              className="min-h-16 w-full flex-1 resize-none rounded-none border-0 bg-transparent p-0 text-sm shadow-none focus:border-transparent focus:ring-0"
              placeholder="Enter to send, Ctrl/Cmd+Enter for newline"
              value={composer}
              onChange={(event) => setComposer(event.target.value)}
              onKeyDown={(event) => {
                if (event.key !== "Enter" || event.nativeEvent.isComposing) return;
                event.preventDefault();
                if (event.metaKey || event.ctrlKey) {
                  setComposer(insertNewlineAtCursor(event.currentTarget));
                  return;
                }
                void handleSend();
              }}
            />
            <div className="mt-2 flex items-center justify-between gap-3">
              <ChatModelSelector
                loading={modelsLoading}
                models={chatModels}
                selectedModelCode={selectedModelCode}
                onSelect={setSelectedModelCode}
              />
              <Button className="shrink-0" disabled={sending || !activeSession || !selectedModel} size="sm" onClick={() => void handleSend()}>
                {sending ? <Wrench size={15} /> : <Send size={15} />}
                Send
              </Button>
            </div>
          </div>
        </footer>
      </main>

    </div>
  );
}
