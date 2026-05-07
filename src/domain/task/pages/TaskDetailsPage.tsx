import { useState } from "react";
import { Plus } from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SelectInput,
  SurfaceCard,
  TronShell,
} from "@/shared";

import {
  useTaskDetail,
  type StatusTone,
} from "@/domain/task/api";
import { cn } from "@/shared/lib/utils";

const badgeToneByStatus = (status: string): StatusTone => {
  const normalized = status.toLowerCase();
  if (normalized.includes("healthy") || normalized.includes("active") || normalized.includes("connected") || normalized.includes("completed")) return "success";
  if (normalized.includes("review") || normalized.includes("sync") || normalized.includes("progress") || normalized.includes("override")) return "warning";
  if (normalized.includes("suspend") || normalized.includes("disabled") || normalized.includes("deprecated")) return "error";
  return "neutral";
};

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function ProgressBar({ value }: { value: number }) {
  return (
    <div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
      <div className="h-full rounded-full bg-[var(--color-primary)]" style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
}

export function TaskDetailsPage() {
  const task = useTaskDetail();
  const [status, setStatus] = useState(task.status);
  const [subtasks, setSubtasks] = useState(task.subtasks);
  const [notice, setNotice] = useState("Task execution context ready.");
  const completed = subtasks.filter((item) => item.done).length;

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search task context...">
      <PageHeader
        breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "Core Infrastructure Update" }, { label: task.id }]}
        title="Task Details"
        actions={<Button onClick={() => {
          setStatus("In Progress");
          setNotice("Task execution queued");
        }}>Execute Task</Button>}
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <SurfaceCard>
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div>
                <div className="mb-2 flex flex-wrap items-center gap-2">
                  <span className="font-mono text-sm text-[var(--color-on-surface-variant)]">{task.id}</span>
                  <SelectInput className="h-8 w-40" value={status} onChange={(event) => setStatus(event.target.value)}>
                    {["To Do", "In Progress", "Review", "Completed"].map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                  <Badge tone="error">{task.priority}</Badge>
                </div>
                <h2 className="text-2xl font-semibold">{task.title}</h2>
              </div>
              <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] px-3 py-2 text-sm">
                <span className="block text-[11px] uppercase text-[var(--color-on-surface-variant)]">Assigned Agent</span>
                <span className="font-medium">{task.agent}</span>
              </div>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <h3 className="border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Description & Requirements</h3>
            <p className="mt-4 text-sm leading-6">{task.description}</p>
            <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-[var(--color-on-surface-variant)]">
              {task.requirements.map((item) => <li key={item}>{item}</li>)}
            </ul>
          </SurfaceCard>
          <div className="grid gap-5 md:grid-cols-2">
            <SurfaceCard><h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Resource Inheritance</h3><div className="rounded-lg bg-[var(--color-surface-container-low)] p-3">Model Config → gpt-4-turbo-config-v2</div></SurfaceCard>
            <SurfaceCard><h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Linked Resources</h3>{task.resources.map((item) => <button className="mb-2 flex w-full rounded-lg border border-transparent p-2 text-left hover:border-[var(--color-outline-variant)] hover:bg-[var(--color-surface-container-low)]" type="button" key={item} onClick={() => setNotice(`${item} resource selected`)}>{item}</button>)}</SurfaceCard>
          </div>
          <SurfaceCard>
            <div className="mb-3 flex items-center justify-between border-b border-[var(--color-outline-variant)] pb-2">
              <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Execution Timeline</h3>
              <Badge tone="accent">Live</Badge>
            </div>
            <div className="space-y-2 rounded-lg bg-[var(--color-surface-container-low)] p-4 font-mono text-[13px]">
              {task.timeline.map((event) => <div className="flex gap-4" key={event.time}><span className="text-[var(--color-on-surface-variant)]">{event.time}</span><span>{event.message}</span></div>)}
            </div>
          </SurfaceCard>
        </div>
        <aside className="space-y-5">
          <SurfaceCard>
            <h3 className="border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Task Metadata</h3>
            <div className="mt-4 space-y-3 text-sm">
              {Object.entries(task.metadata).map(([key, value]) => <div className="flex justify-between gap-3" key={key}><span className="text-[var(--color-on-surface-variant)]">{key}</span><span className="font-medium">{value}</span></div>)}
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <div className="mb-3 flex items-center justify-between border-b border-[var(--color-outline-variant)] pb-2"><h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Sub-tasks</h3><span className="text-xs">{completed}/{subtasks.length} Completed</span></div>
            <div className="space-y-2">
              {subtasks.map((item) => (
                <label className="block rounded-lg p-2 hover:bg-[var(--color-surface-container-low)]" key={item.id}>
                  <div className="flex items-start gap-3">
                    <input type="checkbox" checked={item.done} onChange={() => setSubtasks((rows) => rows.map((row) => row.id === item.id ? { ...row, done: !row.done, status: !row.done ? "Completed" : "Pending", progress: !row.done ? 100 : row.progress } : row))} />
                    <div className="flex-1">
                      <div className={cn("text-sm", item.done ? "text-[var(--color-on-surface-variant)] line-through" : "")}>{item.title}</div>
                      <Badge tone={badgeToneByStatus(item.status)}>{item.status}</Badge>
                      <div className="mt-2"><ProgressBar value={item.progress} /></div>
                    </div>
                  </div>
                </label>
              ))}
            </div>
            <Button className="mt-3 w-full" variant="secondary" onClick={() => {
              const nextIndex = subtasks.length + 1;
              setSubtasks((rows) => [...rows, { id: `sub-${nextIndex}`, title: `Draft follow-up ${nextIndex}`, done: false, status: "Pending", progress: 0 }]);
              setNotice(`Sub-task ${nextIndex} added`);
            }}><Plus size={14} aria-hidden="true" />Add Sub-task</Button>
          </SurfaceCard>
        </aside>
      </div>
    </TronShell>
  );
}
