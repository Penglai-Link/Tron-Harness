import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Plus,
  RefreshCcw,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  useTaskKanbanColumns,
  useTaskOptions,
} from "@/domain/task/hooks";
import type {
  KanbanColumn,
  TaskListItem,
  TaskPriority,
  TaskStatus,
  TaskType,
} from "@/domain/task/model";
import {
  compareTaskListItems,
  inferTaskRole,
  statusLabel,
} from "@/domain/task/model";
import { updateTaskStatus } from "@/domain/task/api";
import { updateMockTaskStatus } from "@/domain/task/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { cn } from "@/shared/lib/utils";
import { SelectItem } from "@/components/ui/select";
import { useTranslation } from "react-i18next";

type BoardScalePreset = "fit" | "100" | "90" | "80" | "70" | "60" | "50";

const BOARD_SCALE_STORAGE_KEY = "tron.tasks.boardScale";
const BOARD_BASE_COLUMN_WIDTH = 320;
const BOARD_BASE_GAP = 16;
const BOARD_BASE_COLUMN_PADDING = 12;
const BOARD_MIN_SCALE = 0.5;
const ROOT_PARENT_TASK_FILTER = "__ROOT__";
const BOARD_SCALE_PRESETS: Array<{ label: string; value: Exclude<BoardScalePreset, "fit"> }> = [
  { label: "100%", value: "100" },
  { label: "90%", value: "90" },
  { label: "80%", value: "80" },
  { label: "70%", value: "70" },
  { label: "60%", value: "60" },
  { label: "50%", value: "50" },
];

function resolveStoredBoardScale(value: string | null): BoardScalePreset {
  if (value === "100" || value === "90" || value === "80" || value === "70" || value === "60" || value === "50") return value;
  return "fit";
}

function presetScaleFactor(preset: BoardScalePreset) {
  if (preset === "fit") return null;
  return Number(preset) / 100;
}

function clampBoardScale(value: number) {
  return Math.min(1, Math.max(BOARD_MIN_SCALE, value));
}

function ProgressBar({ className, value }: { className?: string; value: number }) {
  return (
    <div className={cn("h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]", className)}>
      <div className="h-full rounded-full bg-[var(--color-primary)]" style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
}

function taskTypeMarkerClass(taskType: TaskType) {
  if (taskType === "Bug") return "bg-[var(--color-error)]";
  return "bg-[var(--color-primary)]";
}

function taskPriorityTone(priority: TaskPriority) {
  if (priority === "Extra High" || priority === "High") return "error";
  if (priority === "Medium") return "warning";
  return "outline";
}

function reorderKanbanColumns(
  columns: KanbanColumn[],
  task: TaskListItem,
  nextStatus: TaskStatus,
) {
  return columns.map((column) => {
    if (column.status === task.status && column.status === nextStatus) {
      return column;
    }

    if (column.status === task.status) {
      return {
        ...column,
        tasks: column.tasks.filter((item) => item.taskCode !== task.taskCode),
      };
    }

    if (column.status === nextStatus) {
      const movedTask = {
        ...task,
        status: nextStatus,
      };
      return {
        ...column,
        tasks: [...column.tasks.filter((item) => item.taskCode !== task.taskCode), movedTask].sort(compareTaskListItems),
      };
    }

    return column;
  });
}

function buildTaskDragPreview(card: HTMLButtonElement) {
  const bounds = card.getBoundingClientRect();
  const previewCard = card.cloneNode(true);
  if (!(previewCard instanceof HTMLButtonElement)) return null;

  const preview = document.createElement("div");
  const previewPadding = 18;
  preview.setAttribute("aria-hidden", "true");
  Object.assign(preview.style, {
    background: "transparent",
    left: "-1000px",
    padding: `${previewPadding}px`,
    pointerEvents: "none",
    position: "fixed",
    top: "-1000px",
    width: `${bounds.width + previewPadding * 2}px`,
    zIndex: "9999",
  });

  previewCard.setAttribute("aria-hidden", "true");
  previewCard.tabIndex = -1;
  Object.assign(previewCard.style, {
    background: "var(--color-elevated-surface-strong)",
    borderColor: "var(--color-primary)",
    boxShadow: "0 18px 40px rgba(15, 23, 42, 0.18)",
    margin: "0",
    opacity: "1",
    transform: "translateY(-1px)",
    width: `${bounds.width}px`,
  });

  preview.appendChild(previewCard);
  document.body.appendChild(preview);
  return {
    bounds,
    preview,
    previewOffsetX: previewPadding + Math.min(bounds.width / 2, 96),
    previewOffsetY: previewPadding + 24,
  };
}

function KanbanCard({
  compact,
  dense,
  dragging,
  onDragEnd,
  onDragStart,
  onSelect,
  pending,
  selected,
  task,
}: {
  task: TaskListItem;
  selected: boolean;
  compact: boolean;
  dense: boolean;
  dragging: boolean;
  pending: boolean;
  onDragEnd: () => void;
  onDragStart: (event: React.DragEvent<HTMLButtonElement>) => void;
  onSelect: () => void;
}) {
  const { t } = useTranslation(["pages"]);
  return (
    <button
      aria-busy={pending}
      className={cn(
        "relative z-0 w-full rounded-xl border bg-[var(--color-elevated-surface)] text-left shadow-[0_2px_6px_rgba(15,23,42,0.05)] transition-[box-shadow,border-color,background-color,opacity] duration-150 hover:z-10 hover:border-[var(--color-outline)] hover:shadow-[0_12px_28px_rgba(15,23,42,0.10)]",
        dense ? "p-2" : compact ? "p-2.5" : "p-3",
        selected ? "border-[var(--color-primary)] shadow-[0_10px_24px_rgba(15,23,42,0.08)]" : "border-[var(--color-outline-variant)]",
        dragging ? "translate-y-0 border-dashed border-[var(--color-primary)]/35 bg-[var(--color-primary)]/5 shadow-none hover:translate-y-0 hover:shadow-none" : "",
        pending ? "cursor-wait opacity-70" : "cursor-grab active:cursor-grabbing",
      )}
      data-testid={`task-kanban-card-${task.taskCode}`}
      draggable={!pending}
      type="button"
      onDragEnd={onDragEnd}
      onDragStart={onDragStart}
      onClick={onSelect}
    >
      <div className={cn("flex min-w-0 items-center justify-between gap-[12px]", dense ? "mb-1" : compact ? "mb-1.5" : "mb-2")}>
        <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-[var(--color-on-surface-variant)]">{task.taskCode}</span>
        <Badge className="max-w-[5.5rem] shrink-0 truncate whitespace-nowrap px-2 text-[11px]" title={task.priority} variant={taskPriorityTone(task.priority)}>{task.priority}</Badge>
      </div>
      <div className={cn("flex min-w-0 items-start gap-[12px]", dense ? "mb-1.5" : compact ? "mb-2" : "mb-3")}>
        <span
          aria-label={task.taskType}
          className={cn("mt-1 h-3 w-1 shrink-0 rounded-full", taskTypeMarkerClass(task.taskType))}
          title={task.taskType}
        />
        <h3 className={cn("min-w-0 flex-1 font-medium leading-5", dense ? "text-[12px]" : compact ? "text-[13px]" : "text-sm")}>{task.title}</h3>
      </div>
      <ProgressBar value={task.progress} />
      <div className={cn("flex items-center justify-between gap-[12px]", dense ? "mt-2" : compact ? "mt-2.5" : "mt-3")}>
        <span className="min-w-0 truncate text-[11px] text-[var(--color-on-surface-variant)]">{task.assigneeUserCodes[0] ?? t("pages:tasks.kanban.unassigned", "Unassigned")}</span>
        <span className="text-[11px] text-[var(--color-on-surface-variant)]">{t("pages:tasks.kanban.subtasks", "{{count}} subtasks", { count: task.subtaskCount })}</span>
      </div>
    </button>
  );
}

export function TasksKanbanPage() {
  const { t } = useTranslation(["pages"]);
  const options = useTaskOptions();
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [taskType, setTaskType] = useState<TaskType>("Feature");
  const [priority, setPriority] = useState("");
  const [selectedTaskCode, setSelectedTaskCode] = useState<string | null>(null);
  const [draggedTask, setDraggedTask] = useState<TaskListItem | null>(null);
  const [dragOverState, setDragOverState] = useState<{ allowed: boolean; status: TaskStatus } | null>(null);
  const [pendingTaskCode, setPendingTaskCode] = useState<string | null>(null);
  const [boardScalePreset, setBoardScalePreset] = useState<BoardScalePreset>(() => {
    if (typeof window === "undefined") return "fit";
    return resolveStoredBoardScale(window.localStorage.getItem(BOARD_SCALE_STORAGE_KEY));
  });
  const [boardViewportWidth, setBoardViewportWidth] = useState(0);
  const navigate = useNavigate();
  const toast = useToast();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const boardViewportRef = useRef<HTMLDivElement | null>(null);
  const dragPreviewRef = useRef<HTMLDivElement | null>(null);
  const dataSource = getServiceDataSource("task");
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const workspaceCode = activeWorkspace?.workspaceCode ?? "";

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
    }, 280);

    return () => window.clearTimeout(timer);
  }, [searchInput]);

  const taskQuery = useMemo(() => ({
    parentTaskCode: ROOT_PARENT_TASK_FILTER,
    priority: priority || undefined,
    search: debouncedSearch,
    taskType,
    workspaceCode,
  }), [debouncedSearch, priority, taskType, workspaceCode]);
  const { columns, loading, refresh } = useTaskKanbanColumns(taskQuery);
  const [optimisticColumns, setOptimisticColumns] = useState<KanbanColumn[] | null>(null);

  useEffect(() => {
    setOptimisticColumns(null);
  }, [columns]);

  const displayedColumns = optimisticColumns ?? columns;
  const rootTaskColumns = useMemo(() => (
    displayedColumns.map((column) => ({
      ...column,
      tasks: column.tasks.filter((task) => !task.parentTaskCode),
    }))
  ), [displayedColumns]);

  useEffect(() => {
    setSelectedTaskCode(null);
  }, [taskType, workspaceCode]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(BOARD_SCALE_STORAGE_KEY, boardScalePreset);
  }, [boardScalePreset]);

  useEffect(() => {
    const viewport = boardViewportRef.current;
    if (!viewport) return undefined;

    const updateViewportWidth = () => {
      setBoardViewportWidth(viewport.clientWidth);
    };

    updateViewportWidth();

    const observer = new ResizeObserver((entries) => {
      const nextWidth = entries[0]?.contentRect.width ?? viewport.clientWidth;
      setBoardViewportWidth(nextWidth);
    });

    observer.observe(viewport);
    return () => observer.disconnect();
  }, []);

  const fitBoardScale = useMemo(() => {
    if (!columns.length || boardViewportWidth <= 0) return 1;
    const naturalBoardWidth = columns.length * BOARD_BASE_COLUMN_WIDTH + Math.max(0, columns.length - 1) * BOARD_BASE_GAP;
    return clampBoardScale(boardViewportWidth / naturalBoardWidth);
  }, [boardViewportWidth, columns.length]);

  const boardScale = presetScaleFactor(boardScalePreset) ?? fitBoardScale;
  const boardScalePercent = Math.round(boardScale * 100);
  const boardLayout = useMemo(() => ({
    compact: boardScale < 0.86,
    dense: boardScale < 0.68,
    columnGap: Math.max(8, Math.round(BOARD_BASE_GAP * boardScale)),
    columnPadding: Math.max(6, Math.round(BOARD_BASE_COLUMN_PADDING * boardScale)),
    columnWidth: Math.max(160, Math.round(BOARD_BASE_COLUMN_WIDTH * boardScale)),
  }), [boardScale]);

  const parseDraggedTask = useCallback((event: React.DragEvent<HTMLElement>) => {
    if (draggedTask) return draggedTask;
    const serialized = event.dataTransfer.getData("application/json");
    if (!serialized) return null;
    try {
      return JSON.parse(serialized) as TaskListItem;
    } catch {
      return null;
    }
  }, [draggedTask]);

  const canMoveTaskToStatus = useCallback((task: TaskListItem, status: TaskStatus) => {
    const taskRole = inferTaskRole(task.taskType, task.parentTaskCode);
    return options.statusRules[taskRole].includes(status);
  }, [options.statusRules]);

  const cleanupDragPreview = useCallback(() => {
    dragPreviewRef.current?.remove();
    dragPreviewRef.current = null;
  }, []);

  const clearDragState = useCallback(() => {
    cleanupDragPreview();
    setDraggedTask(null);
    setDragOverState(null);
  }, [cleanupDragPreview]);

  useEffect(() => cleanupDragPreview, [cleanupDragPreview]);

  const handleCardDragStart = useCallback((task: TaskListItem, event: React.DragEvent<HTMLButtonElement>) => {
    if (pendingTaskCode) {
      event.preventDefault();
      return;
    }
    clearDragState();
    setDraggedTask(task);
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", task.taskCode);
    event.dataTransfer.setData("application/json", JSON.stringify(task));
    const dragPreview = buildTaskDragPreview(event.currentTarget);
    if (!dragPreview) return;
    dragPreviewRef.current = dragPreview.preview;
    event.dataTransfer.setDragImage(dragPreview.preview, dragPreview.previewOffsetX, dragPreview.previewOffsetY);
  }, [clearDragState, pendingTaskCode]);

  const handleColumnDragOver = useCallback((status: TaskStatus, event: React.DragEvent<HTMLElement>) => {
    const task = parseDraggedTask(event);
    if (!task || pendingTaskCode) return;
    event.preventDefault();
    if (task.status === status) {
      setDragOverState(null);
      event.dataTransfer.dropEffect = "move";
      return;
    }
    const allowed = canMoveTaskToStatus(task, status);
    event.dataTransfer.dropEffect = "move";
    setDragOverState({ allowed, status });
  }, [canMoveTaskToStatus, parseDraggedTask, pendingTaskCode]);

  const handleColumnDrop = useCallback(async (status: TaskStatus, event: React.DragEvent<HTMLElement>) => {
    event.preventDefault();
    setDragOverState(null);

    const task = parseDraggedTask(event);
    cleanupDragPreview();
    setDraggedTask(null);
    if (!task || pendingTaskCode) return;
    if (task.status === status) return;

    if (!canMoveTaskToStatus(task, status)) {
      toast.warning(`Task ${task.taskCode} cannot move to ${statusLabel(status)}`);
      return;
    }

    setPendingTaskCode(task.taskCode);
    try {
      if (dataSource === "api") {
        await updateTaskStatus(task.taskCode, { status });
      } else {
        updateMockTaskStatus(task.taskCode, { status });
      }
      setOptimisticColumns(reorderKanbanColumns(displayedColumns, task, status));
      refresh();
      toast.success(`Task ${task.taskCode} moved to ${statusLabel(status)}`);
    } catch (error) {
      setOptimisticColumns(null);
      refresh();
      toast.error(error instanceof Error ? error.message : t("pages:tasks.kanban.errors.status", "Task status could not be updated"));
    } finally {
      setPendingTaskCode(null);
    }
  }, [canMoveTaskToStatus, cleanupDragPreview, dataSource, displayedColumns, parseDraggedTask, pendingTaskCode, refresh, toast]);

  return (
    <div className="flex min-h-[calc(100vh-6.5rem)] flex-col">
      <PageHeader
        title={t("pages:tasks.kanban.title", "Task Management")}
        actions={
          <>
            <SelectInput aria-label={t("pages:tasks.kanban.scale", "Task board scale")} className="w-[128px] bg-white" value={boardScalePreset} onValueChange={(event) => setBoardScalePreset(event as BoardScalePreset)}>
              <SelectItem value="fit">{`Fit (${boardScalePercent}%)`}</SelectItem>
              {BOARD_SCALE_PRESETS.map((preset) => <SelectItem key={preset.value} value={preset.value}>{preset.label}</SelectItem>)}
            </SelectInput>
            <Button size="lg" variant="outline" onClick={() => {
              refresh();
              toast.success(t("pages:tasks.kanban.messages.refreshed", "Task board refreshed"));
            }}>{t("pages:tasks.actions.refresh", "Refresh")}</Button>
            <Button size="lg" onClick={() => navigate("/tasks/new")}>{t("pages:tasks.actions.new", "New")}</Button>
          </>
        }
      />
      <SurfaceCard className="mb-4 grid gap-3 p-4 md:grid-cols-[minmax(0,1fr)_180px_220px]">
        <div className="grid gap-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("pages:tasks.kanban.search", "Search")}</span>
          <SearchField placeholder={t("pages:tasks.kanban.searchPlaceholder", "Filter task code, title, or description...")} value={searchInput} onChange={setSearchInput} />
        </div>
        <div className="grid gap-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("pages:tasks.kanban.taskType", "Task Type")}</span>
          <SelectInput aria-label={t("pages:tasks.kanban.taskTypeFilter", "Task type filter")} value={taskType} onValueChange={(event) => setTaskType(event as TaskType)}>
            {options.taskTypes.map((item) => <SelectItem key={item} value={item}>{item}</SelectItem>)}
          </SelectInput>
        </div>
        <div className="grid gap-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("pages:tasks.kanban.priority", "Priority")}</span>
          <SelectInput aria-label={t("pages:tasks.kanban.priorityFilter", "Task priority filter")} value={priority} onValueChange={(event) => setPriority(event!)}>
            <SelectItem value="">{t("pages:tasks.kanban.allPriorities", "All priorities")}</SelectItem>
            {options.priorities.map((item) => <SelectItem key={item} value={item}>{item}</SelectItem>)}
          </SelectInput>
        </div>
      </SurfaceCard>
      <div className="flex min-h-0 flex-1 overflow-x-auto pb-4" ref={boardViewportRef} style={{ gap: `${boardLayout.columnGap}px` }}>
        {rootTaskColumns.map((column) => (
          <section
            aria-label={`${column.title} task column`}
            className={cn(
              "flex shrink-0 flex-col rounded-2xl bg-[var(--color-surface-container)] transition-[background-color,box-shadow,border-color] duration-150",
              dragOverState?.status === column.status
                ? dragOverState.allowed
                  ? "bg-[var(--color-primary)]/6 ring-1 ring-inset ring-[var(--color-primary)]/20"
                  : "bg-[var(--color-error-container)]/40 ring-1 ring-inset ring-[var(--color-error)]/15"
                : "",
            )}
            data-task-kanban-column={column.status}
            data-testid={`task-kanban-dropzone-${column.status}`}
            key={column.status}
            onDragLeave={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
                setDragOverState(null);
              }
            }}
            onDragOver={(event) => handleColumnDragOver(column.status, event)}
            onDrop={(event) => handleColumnDrop(column.status, event)}
            style={{
              gap: `${boardLayout.columnGap}px`,
              padding: `${boardLayout.columnPadding}px`,
              width: `${boardLayout.columnWidth}px`,
            }}
          >
            <div className="flex items-center justify-between border-b-2 border-[var(--color-outline-variant)] px-1 py-1">
              <h2 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{column.title}</h2>
              <span
                className={cn(
                  "rounded-full px-2.5 py-0.5 text-[11px] transition-colors",
                  dragOverState?.status === column.status
                    ? dragOverState.allowed
                      ? "bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                      : "bg-[var(--color-error-container)] text-[var(--color-on-error-container)]"
                    : "bg-[var(--color-surface-container-high)]",
                )}
              >
                {column.tasks.length}
              </span>
            </div>
            <div
              className="min-h-0 flex-1 space-y-3 overflow-y-auto"
              onDragOver={(event) => handleColumnDragOver(column.status, event)}
              onDrop={(event) => handleColumnDrop(column.status, event)}
            >
              {dragOverState?.status === column.status ? (
                <div
                  className={cn(
                    "rounded-xl border border-dashed px-3 py-2 text-[11px] font-medium transition-colors",
                    dragOverState.allowed
                      ? "border-[var(--color-primary)]/35 bg-[var(--color-elevated-surface-strong)] text-[var(--color-primary)]"
                      : "border-[var(--color-error)]/30 bg-[var(--color-error-container)]/45 text-[var(--color-error)]",
                  )}
                >
                  {dragOverState.allowed
                    ? `Release to move into ${column.title}`
                    : `${draggedTask?.taskCode ?? "Task"} cannot move into ${column.title}`}
                </div>
              ) : null}
              {column.tasks.map((task) => (
                <KanbanCard
                  key={task.taskCode}
                  task={task}
                  compact={boardLayout.compact}
                  dense={boardLayout.dense}
                  dragging={draggedTask?.taskCode === task.taskCode}
                  pending={pendingTaskCode === task.taskCode}
                  selected={selectedTaskCode === task.taskCode}
                  onDragEnd={() => {
                    clearDragState();
                  }}
                  onDragStart={(event) => handleCardDragStart(task, event)}
                  onSelect={() => {
                    setSelectedTaskCode(task.taskCode);
                    navigate(`/tasks/detail?taskCode=${encodeURIComponent(task.taskCode)}`);
                  }}
                />
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
