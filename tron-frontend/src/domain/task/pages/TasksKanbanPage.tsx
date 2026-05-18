import {
  useMemo,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Plus,
  RefreshCcw,
  Search,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextInput,
  TronShell,
  useToast,
} from "@/shared";

import {
  useTaskKanbanColumns,
  useTaskOptions,
  useTaskWorkspaceOptions,
} from "@/domain/task/hooks";
import type {
  TaskListItem,
  TaskSourceType,
  TaskType,
} from "@/domain/task/model";
import { cn } from "@/shared/lib/utils";

const DEFAULT_WORKSPACE_CODE = "TRON_CORE";

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput aria-label="Search tasks" className="w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function ProgressBar({ value }: { value: number }) {
  return (
    <div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
      <div className="h-full rounded-full bg-[var(--color-primary)]" style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
}

function statusLabel(value: string) {
  if (value === "InProgress") return "In Progress";
  if (value === "Todo") return "To Do";
  return value;
}

function taskTypeTone(taskType: TaskType) {
  if (taskType === "BugFix") return "error";
  if (taskType === "Refactor") return "warning";
  return "accent";
}

function sourceTypeTone(sourceType: TaskSourceType) {
  if (sourceType === "Internal") return "neutral";
  return "success";
}

function KanbanCard({ task, selected, onSelect }: { task: TaskListItem; selected: boolean; onSelect: () => void }) {
  return (
    <button className={cn("w-full rounded-lg border bg-white p-3 text-left shadow-sm hover:border-[var(--color-outline)]", selected ? "border-[var(--color-primary)]" : "border-[var(--color-outline-variant)]")} type="button" onClick={onSelect}>
      <div className="mb-2 flex items-center justify-between">
        <span className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{task.taskCode}</span>
        <Badge tone={task.priority === "High" ? "error" : task.priority === "Medium" ? "warning" : "neutral"}>{task.priority}</Badge>
      </div>
      <h3 className="mb-3 text-sm font-medium leading-5">{task.title}</h3>
      <div className="mb-3 flex flex-wrap gap-1.5">
        <Badge tone={taskTypeTone(task.taskType)}>{task.taskType}</Badge>
        <Badge tone={sourceTypeTone(task.sourceType)}>{task.sourceType}</Badge>
      </div>
      <ProgressBar value={task.progress} />
      <div className="mt-3 flex items-center justify-between">
        <span className="text-[11px] text-[var(--color-on-surface-variant)]">{task.assigneeUserCodes[0] ?? "Unassigned"}</span>
        <span className="text-[11px] text-[var(--color-on-surface-variant)]">{task.subtaskCount} subtasks</span>
      </div>
    </button>
  );
}

export function TasksKanbanPage() {
  const options = useTaskOptions();
  const [workspaceCode, setWorkspaceCode] = useState(DEFAULT_WORKSPACE_CODE);
  const [query, setQuery] = useState("");
  const [priority, setPriority] = useState("");
  const [selectedTaskCode, setSelectedTaskCode] = useState<string | null>(null);
  const navigate = useNavigate();
  const toast = useToast();
  const { workspaces } = useTaskWorkspaceOptions(workspaceCode);
  const taskQuery = useMemo(() => ({
    priority: priority || undefined,
    search: query,
    workspaceCode,
  }), [priority, query, workspaceCode]);
  const { columns, loading, refresh } = useTaskKanbanColumns(taskQuery);

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search tasks...">
      <PageHeader
        title="Task Management"
        status={loading ? "Loading" : selectedTaskCode ?? undefined}
        actions={
          <>
            <Button variant="secondary" onClick={() => {
              refresh();
              toast.success("Task board refreshed");
            }}><RefreshCcw size={16} aria-hidden="true" />Refresh</Button>
            <Button onClick={() => navigate("/tasks/new")}><Plus size={16} aria-hidden="true" />Create Task</Button>
          </>
        }
      />
      <SurfaceCard className="mb-4 grid gap-3 p-4 md:grid-cols-[240px_minmax(0,1fr)_220px]">
        <Field label="Workspace">
          <SelectInput aria-label="Task workspace" value={workspaceCode} onChange={(event) => setWorkspaceCode(event.target.value)}>
            {workspaces.map((workspace) => (
              <option key={workspace.workspaceCode} value={workspace.workspaceCode}>
                {workspace.label}
              </option>
            ))}
          </SelectInput>
        </Field>
        <Field label="Search">
          <SearchField placeholder="Filter task code, title, or description..." value={query} onChange={setQuery} />
        </Field>
        <Field label="Priority">
          <SelectInput aria-label="Task priority filter" value={priority} onChange={(event) => setPriority(event.target.value)}>
            <option value="">All priorities</option>
            {options.priorities.map((item) => <option key={item} value={item}>{item}</option>)}
          </SelectInput>
        </Field>
      </SurfaceCard>
      <div className="flex h-[720px] gap-4 overflow-x-auto pb-4">
        {columns.map((column) => (
          <section className="flex w-[320px] shrink-0 flex-col gap-3 rounded-lg bg-[var(--color-surface-container)] p-3" key={column.status}>
            <div className="flex items-center justify-between border-b-2 border-[var(--color-outline-variant)] px-1 py-1">
              <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{column.title}</span>
              <span className="rounded bg-[var(--color-surface-container-high)] px-2 text-[11px]">{column.tasks.length}</span>
            </div>
            <div className="space-y-3 overflow-y-auto">
              {column.tasks.map((task) => (
                <KanbanCard
                  key={task.taskCode}
                  task={task}
                  selected={selectedTaskCode === task.taskCode}
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
    </TronShell>
  );
}
