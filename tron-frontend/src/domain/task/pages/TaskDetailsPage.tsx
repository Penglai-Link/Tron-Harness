import { useEffect, useState } from "react";
import { useSearchParams } from "react-router";
import {
  Archive,
  Plus,
  RotateCcw,
  Save,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  TronShell,
  useToast,
} from "@/shared";

import {
  archiveTask,
  createTask,
  restoreTask,
  updateTask,
  updateTaskStatus,
} from "@/domain/task/api";
import {
  archiveMockTask,
  createMockTask,
  restoreMockTask,
  updateMockTask,
  updateMockTaskStatus,
} from "@/domain/task/mock/data";
import {
  useTaskDetail,
  useTaskOptions,
  useTaskWorkspaceOptions,
} from "@/domain/task/hooks";
import type {
  TaskPriority,
  TaskSourceType,
  TaskStatus,
  TaskType,
} from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

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

function toggleCode(codes: string[], code: string) {
  return codes.includes(code) ? codes.filter((item) => item !== code) : [...codes, code];
}

function splitCodes(value: string) {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
}

export function TaskDetailsPage() {
  const [searchParams] = useSearchParams();
  const taskCode = searchParams.get("taskCode") || "TSK-SEED-BACKLOG";
  const { task, setTask, refresh, loading } = useTaskDetail(taskCode);
  const options = useTaskOptions();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const archived = task.status === "Archived" || Boolean(task.archivedAt);
  const { effectiveUsers } = useTaskWorkspaceOptions(task.workspaceCode);
  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description ?? "");
  const [taskType, setTaskType] = useState<TaskType>(task.taskType);
  const [sourceType, setSourceType] = useState<TaskSourceType>(task.sourceType);
  const [sourceUrl, setSourceUrl] = useState(task.sourceUrl ?? "");
  const [priority, setPriority] = useState<TaskPriority>(task.priority);
  const [status, setStatus] = useState<TaskStatus>(task.status);
  const [progress, setProgress] = useState(String(task.progress));
  const [agentCodesInput, setAgentCodesInput] = useState(task.agentCodes.join(", "));
  const [appCodesInput, setAppCodesInput] = useState(task.appCodes.join(", "));
  const [assigneeUserCodes, setAssigneeUserCodes] = useState(task.assigneeUserCodes);
  const [childTitle, setChildTitle] = useState("Follow-up subtask");

  useEffect(() => {
    setTitle(task.title);
    setDescription(task.description ?? "");
    setTaskType(task.taskType);
    setSourceType(task.sourceType);
    setSourceUrl(task.sourceUrl ?? "");
    setPriority(task.priority);
    setStatus(task.status);
    setProgress(String(task.progress));
    setAgentCodesInput(task.agentCodes.join(", "));
    setAppCodesInput(task.appCodes.join(", "));
    setAssigneeUserCodes(task.assigneeUserCodes);
  }, [task]);

  async function saveTask() {
    if (archived) {
      toast.warning("Restore archived task before editing");
      return;
    }
    try {
      const payload = {
        agentCodes: splitCodes(agentCodesInput),
        appCodes: splitCodes(appCodesInput),
        assigneeUserCodes,
        description,
        priority,
        sourceType,
        sourceUrl,
        taskType,
        title,
      };
      const updated = dataSource === "api" ? await updateTask(task.taskCode, payload) : updateMockTask(task.taskCode, payload);
      setTask(updated);
      toast.success(`Task ${updated.taskCode} updated`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Task could not be updated");
    }
  }

  async function saveStatus() {
    if (archived) {
      toast.warning("Restore archived task before changing status");
      return;
    }
    try {
      const payload = {
        progress: Number(progress),
        status,
      };
      const updated = dataSource === "api" ? await updateTaskStatus(task.taskCode, payload) : updateMockTaskStatus(task.taskCode, payload);
      setTask(updated);
      toast.success(`Task ${updated.taskCode} status updated`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Task status could not be updated");
    }
  }

  async function addSubtask() {
    if (archived) {
      toast.warning("Restore archived task before adding subtasks");
      return;
    }
    try {
      const payload = {
        agentCodes: [],
        appCodes: [],
        assigneeUserCodes: assigneeUserCodes.slice(0, 1),
        description: `Subtask for ${task.taskCode}`,
        parentTaskCode: task.taskCode,
        priority: "Low" as TaskPriority,
        sourceType: "Internal" as TaskSourceType,
        sourceUrl: "",
        taskType: "Etc" as TaskType,
        title: childTitle,
        workspaceCode: task.workspaceCode,
      };
      const child = dataSource === "api"
        ? await createTask(payload)
        : createMockTask(payload);
      refresh();
      toast.success(`Subtask ${child.taskCode} created`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Subtask could not be created");
    }
  }

  async function toggleArchive() {
    try {
      if (archived) {
        await (dataSource === "api" ? restoreTask(task.taskCode) : restoreMockTask(task.taskCode));
        toast.success(`Task ${task.taskCode} restored`);
      } else {
        await (dataSource === "api" ? archiveTask(task.taskCode) : archiveMockTask(task.taskCode));
        toast.success(`Task ${task.taskCode} archived`);
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Task lifecycle action failed");
    }
  }

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search task context...">
      <PageHeader
        breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: task.taskCode }]}
        title="Task Details"
        status={loading ? "Loading" : archived ? "Archived" : statusLabel(task.status)}
        actions={
          <>
            <Button variant="secondary" onClick={() => {
              toast.info("Task execution is not available in v1");
            }}>Execute Task</Button>
            <Button variant="secondary" onClick={toggleArchive}>
              {archived ? <RotateCcw size={16} aria-hidden="true" /> : <Archive size={16} aria-hidden="true" />}
              {archived ? "Restore" : "Archive"}
            </Button>
          </>
        }
      />
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <SurfaceCard>
            <div className="mb-4 flex flex-wrap items-center gap-2">
              <span className="font-mono text-sm text-[var(--color-on-surface-variant)]">{task.taskCode}</span>
              <Badge tone={archived ? "neutral" : "accent"}>{archived ? "Archived" : statusLabel(task.status)}</Badge>
              <Badge tone={task.priority === "High" ? "error" : task.priority === "Medium" ? "warning" : "neutral"}>{task.priority}</Badge>
              <Badge tone="neutral">{task.taskType}</Badge>
              <Badge tone="neutral">{task.sourceType}</Badge>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Title">
                <TextInput aria-label="Task detail title" disabled={archived} value={title} onChange={(event) => setTitle(event.target.value)} />
              </Field>
              <Field label="Priority">
                <SelectInput aria-label="Task detail priority" disabled={archived} value={priority} onChange={(event) => setPriority(event.target.value as TaskPriority)}>
                  {options.priorities.map((item) => <option key={item}>{item}</option>)}
                </SelectInput>
              </Field>
            </div>
            <div className="mt-4 grid gap-4 md:grid-cols-2">
              <Field label="Task Type">
                <SelectInput aria-label="Task detail type" disabled={archived} value={taskType} onChange={(event) => setTaskType(event.target.value as TaskType)}>
                  {options.taskTypes.map((item) => <option key={item}>{item}</option>)}
                </SelectInput>
              </Field>
              <Field label="Source Type">
                <SelectInput aria-label="Task detail source type" disabled={archived} value={sourceType} onChange={(event) => setSourceType(event.target.value as TaskSourceType)}>
                  {options.sourceTypes.map((item) => <option key={item}>{item}</option>)}
                </SelectInput>
              </Field>
              <Field label="Source URL">
                <TextInput aria-label="Task detail source URL" disabled={archived} placeholder="Optional GitHub/GitLab URL" value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
              </Field>
              <Field label="Agent Codes">
                <TextInput aria-label="Task detail agent codes" disabled={archived} placeholder="agent-reviewer, agent-builder" value={agentCodesInput} onChange={(event) => setAgentCodesInput(event.target.value)} />
              </Field>
              <Field label="App Codes">
                <TextInput aria-label="Task detail app codes" disabled={archived} placeholder="github, notion" value={appCodesInput} onChange={(event) => setAppCodesInput(event.target.value)} />
              </Field>
            </div>
            <div className="mt-4">
              <Field label="Description">
                <TextArea aria-label="Task detail description" className="min-h-36" disabled={archived} value={description} onChange={(event) => setDescription(event.target.value)} />
              </Field>
            </div>
            <SurfaceCard className="mt-4 bg-[var(--color-surface-container-lowest)]">
              <h3 className="mb-3 text-sm font-semibold">Assigned Users</h3>
              <div className="grid gap-2 md:grid-cols-2">
                {effectiveUsers.map((user) => (
                  <label className="flex items-start gap-3 rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={user.userCode}>
                    <input
                      aria-label={`Assign ${user.displayName}`}
                      checked={assigneeUserCodes.includes(user.userCode)}
                      className="mt-1"
                      disabled={archived}
                      type="checkbox"
                      onChange={() => setAssigneeUserCodes((current) => toggleCode(current, user.userCode))}
                    />
                    <span>
                      <span className="block font-medium">{user.displayName}</span>
                      <span className="block text-xs text-[var(--color-on-surface-variant)]">{user.userCode}</span>
                    </span>
                  </label>
                ))}
                {!effectiveUsers.length ? <p className="text-sm text-[var(--color-on-surface-variant)]">No effective users available for this workspace.</p> : null}
              </div>
            </SurfaceCard>
            <Button className="mt-4" disabled={archived} onClick={saveTask}><Save size={15} />Save Details</Button>
          </SurfaceCard>

          <SurfaceCard>
            <h3 className="mb-4 border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Status & Progress</h3>
            <div className="grid gap-4 lg:grid-cols-2 2xl:grid-cols-[200px_180px_160px_auto]">
              <Field label="Status">
                <SelectInput aria-label="Task detail status" disabled={archived} value={status} onChange={(event) => setStatus(event.target.value as TaskStatus)}>
                  {options.statuses.filter((item) => item !== "Archived").map((item) => <option key={item} value={item}>{statusLabel(item)}</option>)}
                </SelectInput>
              </Field>
              <Field label="Progress">
                <TextInput
                  aria-label="Task detail progress"
                  disabled={archived}
                  max="100"
                  min="0"
                  type="number"
                  value={progress}
                  onChange={(event) => setProgress(event.target.value)}
                />
              </Field>
              <div className="flex items-end">
                <Button disabled={archived} onClick={saveStatus}>Update Status</Button>
              </div>
            </div>
            <div className="mt-4"><ProgressBar value={Number(progress)} /></div>
          </SurfaceCard>
        </div>
        <aside className="space-y-5">
          <SurfaceCard>
            <h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Task Metadata</h3>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Workspace</span><span className="font-medium">{task.workspaceCode}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Parent</span><span className="font-medium">{task.parentTaskCode ?? "None"}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Created</span><span className="font-medium">{task.createdAt}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Updated</span><span className="font-medium">{task.updatedAt}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Archived At</span><span className="font-medium">{task.archivedAt ?? "Not archived"}</span></div>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <h3 className="mb-3 border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Subtasks</h3>
            <div className="space-y-2">
              {task.subtasks.map((item) => (
                <div className="rounded-lg border border-[var(--color-outline-variant)] p-3" key={item.taskCode}>
                  <div className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{item.taskCode}</div>
                  <div className="mt-1 text-sm font-medium">{item.title}</div>
                  <div className="mt-2 flex items-center justify-between gap-3">
                    <Badge tone={item.status === "Archived" ? "neutral" : "accent"}>{item.status === "Archived" ? "Archived" : statusLabel(item.status)}</Badge>
                    <span className="text-xs text-[var(--color-on-surface-variant)]">{item.progress}%</span>
                  </div>
                  <div className="mt-2"><ProgressBar value={item.progress} /></div>
                </div>
              ))}
            </div>
            <div className="mt-4">
              <Field label="New Subtask Title">
                <TextInput aria-label="New subtask title" disabled={archived} value={childTitle} onChange={(event) => setChildTitle(event.target.value)} />
              </Field>
            </div>
            <Button className="mt-3 w-full" disabled={archived || !childTitle.trim()} variant="secondary" onClick={addSubtask}><Plus size={14} aria-hidden="true" />Add Subtask</Button>
          </SurfaceCard>
        </aside>
      </div>
    </TronShell>
  );
}
