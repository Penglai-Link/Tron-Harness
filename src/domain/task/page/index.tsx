import { useState } from "react";
import {
  Plus,
  RefreshCcw,
  Search,
  Upload,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/components/tron";
import { useNavigate } from "@/router/client-router";

import {
  useKanbanColumns,
  type KanbanTask,
} from "@/domain/task/api";
import { cn } from "@/lib/utils";

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
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

function KanbanCard({ task, selected, onSelect }: { task: KanbanTask; selected: boolean; onSelect: () => void }) {
  return (
    <button className={cn("w-full rounded-lg border bg-white p-3 text-left shadow-sm hover:border-[var(--color-outline)]", selected ? "border-[var(--color-primary)]" : "border-[var(--color-outline-variant)]")} type="button" onClick={onSelect}>
      <div className="mb-2 flex items-center justify-between">
        <span className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{task.id}</span>
        <Badge tone={task.priority === "HIGH" ? "error" : task.priority === "MED" ? "warning" : "neutral"}>{task.priority}</Badge>
      </div>
      <h3 className="mb-3 text-sm font-medium leading-5">{task.title}</h3>
      <ProgressBar value={task.progress} />
      <div className="mt-3 flex items-center justify-between">
        <span className="flex size-6 items-center justify-center rounded-full bg-[var(--color-surface-container-high)] text-[10px] font-semibold">{task.owner}</span>
        <span className="text-[11px] text-[var(--color-on-surface-variant)]">{task.signal}</span>
      </div>
    </button>
  );
}

export function TasksKanbanPage() {
  const columns = useKanbanColumns();
  const [query, setQuery] = useState("");
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);
  const [notice, setNotice] = useState("Kanban board ready.");
  const navigate = useNavigate();
  const visibleColumns = columns.map((column) => ({ ...column, tasks: column.tasks.filter((task) => task.title.toLowerCase().includes(query.toLowerCase()) || task.id.toLowerCase().includes(query.toLowerCase())) }));

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search tasks...">
      <PageHeader
        title="Task Management"
        status={selectedTask ? selectedTask.id : "Kanban"}
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice("Kanban sync completed")}><RefreshCcw size={16} aria-hidden="true" />Sync</Button>
            <Button variant="secondary" onClick={() => navigate("/tasks/import")}><Upload size={16} aria-hidden="true" />Import Task</Button>
            <Button onClick={() => navigate("/tasks/new")}><Plus size={16} aria-hidden="true" />Create Task</Button>
          </>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <SurfaceCard className="mb-4 p-4">
        <SearchField placeholder="Filter kanban cards..." value={query} onChange={setQuery} />
      </SurfaceCard>
      <div className="flex h-[720px] gap-4 overflow-x-auto pb-4">
        {visibleColumns.map((column) => (
          <section className="flex w-[320px] shrink-0 flex-col gap-3 rounded-lg bg-[var(--color-surface-container)] p-3" key={column.id}>
            <div className="flex items-center justify-between border-b-2 px-1 py-1" style={{ borderColor: column.accent }}>
              <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{column.title}</span>
              <span className="rounded bg-[var(--color-surface-container-high)] px-2 text-[11px]">{column.tasks.length}</span>
            </div>
            <div className="space-y-3 overflow-y-auto">
              {column.tasks.map((task) => (
                <KanbanCard
                  key={task.id}
                  task={task}
                  selected={selectedTask?.id === task.id}
                  onSelect={() => {
                    setSelectedTask(task);
                    navigate("/tasks/detail");
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
