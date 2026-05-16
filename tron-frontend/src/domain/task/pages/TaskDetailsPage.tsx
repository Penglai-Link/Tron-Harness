import {
  useEffect,
  useState,
  type ReactNode,
} from "react";
import {
  Link,
  useSearchParams,
} from "react-router";
import {
  Archive,
  Edit3,
  Plus,
  RotateCcw,
  Save,
  X,
} from "lucide-react";

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

function progressValue(value: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.min(100, Math.max(0, parsed));
}

function taskDetailHref(taskCode: string) {
  return `/tasks/detail?taskCode=${encodeURIComponent(taskCode)}`;
}

function ReadValue({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm leading-6 text-[var(--color-on-surface)]">
      {children || <span className="text-[var(--color-on-surface-variant)]">None</span>}
    </div>
  );
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
  const [editing, setEditing] = useState(false);

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
    setEditing(false);
  }, [task]);

  const effectiveUserByCode = new Map(effectiveUsers.map((user) => [user.userCode, user]));
  const assignedUsers = assigneeUserCodes.map((userCode) => effectiveUserByCode.get(userCode)?.displayName ?? userCode);
  const editable = editing && !archived;

  function resetForm() {
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
    setEditing(false);
  }

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
      let updated = dataSource === "api" ? await updateTask(task.taskCode, payload) : updateMockTask(task.taskCode, payload);
      const nextProgress = progressValue(progress);
      if (status !== task.status || nextProgress !== task.progress) {
        updated = dataSource === "api"
          ? await updateTaskStatus(task.taskCode, { progress: nextProgress, status })
          : updateMockTaskStatus(task.taskCode, { progress: nextProgress, status });
      }
      setTask(updated);
      setEditing(false);
      toast.success(`Task ${updated.taskCode} saved`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Task could not be updated");
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
        taskType: "Feature" as TaskType,
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
        description={loading ? "Loading task data..." : undefined}
        actions={
          <>
            {editable ? (
              <>
                <Button onClick={saveTask}><Save size={15} />Save</Button>
                <Button variant="secondary" onClick={resetForm}><X size={15} />Cancel</Button>
              </>
            ) : !archived ? (
              <Button variant="secondary" onClick={() => setEditing(true)}><Edit3 size={15} />Edit</Button>
            ) : null}
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
            <div className="mb-4">
              <span className="font-mono text-sm text-[var(--color-on-surface-variant)]">{task.taskCode}</span>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Title">
                {editable ? (
                  <TextInput aria-label="Task detail title" value={title} onChange={(event) => setTitle(event.target.value)} />
                ) : (
                  <ReadValue>{task.title}</ReadValue>
                )}
              </Field>
              <Field label="Status">
                {editable ? (
                  <SelectInput aria-label="Task detail status" value={status} onChange={(event) => setStatus(event.target.value as TaskStatus)}>
                    {options.statuses.filter((item) => item !== "Archived").map((item) => <option key={item} value={item}>{statusLabel(item)}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{archived ? "Archived" : statusLabel(task.status)}</ReadValue>
                )}
              </Field>
              <Field label="Priority">
                {editable ? (
                  <SelectInput aria-label="Task detail priority" value={priority} onChange={(event) => setPriority(event.target.value as TaskPriority)}>
                    {options.priorities.map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{task.priority}</ReadValue>
                )}
              </Field>
              <Field label="Progress">
                {editable ? (
                  <TextInput
                    aria-label="Task detail progress"
                    max="100"
                    min="0"
                    type="number"
                    value={progress}
                    onChange={(event) => setProgress(event.target.value)}
                  />
                ) : (
                  <ReadValue>{task.progress}%</ReadValue>
                )}
              </Field>
              <Field label="Task Type">
                {editable ? (
                  <SelectInput aria-label="Task detail type" value={taskType} onChange={(event) => setTaskType(event.target.value as TaskType)}>
                    {options.taskTypes.map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{task.taskType}</ReadValue>
                )}
              </Field>
              <Field label="Source Type">
                {editable ? (
                  <SelectInput aria-label="Task detail source type" value={sourceType} onChange={(event) => setSourceType(event.target.value as TaskSourceType)}>
                    {options.sourceTypes.map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{task.sourceType}</ReadValue>
                )}
              </Field>
              <Field label="Source URL">
                {editable ? (
                  <TextInput aria-label="Task detail source URL" placeholder="Optional GitHub/GitLab URL" value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
                ) : (
                  <ReadValue>{task.sourceUrl || "None"}</ReadValue>
                )}
              </Field>
              <Field label="Agent Codes">
                {editable ? (
                  <TextInput aria-label="Task detail agent codes" placeholder="agent-reviewer, agent-builder" value={agentCodesInput} onChange={(event) => setAgentCodesInput(event.target.value)} />
                ) : (
                  <ReadValue>{task.agentCodes.join(", ") || "None"}</ReadValue>
                )}
              </Field>
              <Field label="App Codes">
                {editable ? (
                  <TextInput aria-label="Task detail app codes" placeholder="github, notion" value={appCodesInput} onChange={(event) => setAppCodesInput(event.target.value)} />
                ) : (
                  <ReadValue>{task.appCodes.join(", ") || "None"}</ReadValue>
                )}
              </Field>
            </div>
            <div className="mt-4"><ProgressBar value={editable ? progressValue(progress) : task.progress} /></div>
            <div className="mt-4">
              <Field label="Description">
                {editable ? (
                  <TextArea aria-label="Task detail description" className="min-h-36 w-full" value={description} onChange={(event) => setDescription(event.target.value)} />
                ) : (
                  <ReadValue>{task.description || "None"}</ReadValue>
                )}
              </Field>
            </div>
            <SurfaceCard className="mt-4 bg-[var(--color-surface-container-lowest)]">
              <h3 className="mb-3 text-sm font-semibold">Assigned Users</h3>
              {editable ? (
                <div className="grid gap-2 md:grid-cols-2">
                  {effectiveUsers.map((user) => (
                    <label className="flex items-start gap-3 rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={user.userCode}>
                      <input
                        aria-label={`Assign ${user.displayName}`}
                        checked={assigneeUserCodes.includes(user.userCode)}
                        className="mt-1"
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
              ) : (
                <ReadValue>{assignedUsers.join(", ") || "None"}</ReadValue>
              )}
            </SurfaceCard>
          </SurfaceCard>
        </div>
        <aside className="space-y-5">
          <SurfaceCard>
            <h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Task Metadata</h3>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Workspace</span><span className="font-medium">{task.workspaceCode}</span></div>
              <div className="flex justify-between gap-3">
                <span className="text-[var(--color-on-surface-variant)]">Parent</span>
                {task.parent ? (
                  <Link className="text-right font-medium text-[var(--color-primary)] hover:underline" to={taskDetailHref(task.parent.taskCode)}>
                    {task.parent.taskCode}
                  </Link>
                ) : (
                  <span className="font-medium">None</span>
                )}
              </div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Created</span><span className="font-medium">{task.createdAt}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Updated</span><span className="font-medium">{task.updatedAt}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Archived At</span><span className="font-medium">{task.archivedAt ?? "Not archived"}</span></div>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <h3 className="mb-3 border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Subtasks</h3>
            <div className="space-y-2">
              {task.subtasks.map((item) => (
                <Link className="block rounded-lg border border-[var(--color-outline-variant)] p-3 hover:border-[var(--color-primary)]" key={item.taskCode} to={taskDetailHref(item.taskCode)}>
                  <div className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{item.taskCode}</div>
                  <div className="mt-1 text-sm font-medium">{item.title}</div>
                  <div className="mt-2 flex items-center justify-between gap-3">
                    <span className="text-xs text-[var(--color-on-surface-variant)]">{statusLabel(item.status)}</span>
                    <span className="text-xs text-[var(--color-on-surface-variant)]">{item.progress}%</span>
                  </div>
                  <div className="mt-2"><ProgressBar value={item.progress} /></div>
                </Link>
              ))}
              {!task.subtasks.length ? <p className="text-sm text-[var(--color-on-surface-variant)]">No active subtasks.</p> : null}
            </div>
            {editable ? (
              <>
                <div className="mt-4">
                  <Field label="New Subtask Title">
                    <TextInput aria-label="New subtask title" value={childTitle} onChange={(event) => setChildTitle(event.target.value)} />
                  </Field>
                </div>
                <Button className="mt-3 w-full" disabled={!childTitle.trim()} variant="secondary" onClick={addSubtask}><Plus size={14} aria-hidden="true" />Add Subtask</Button>
              </>
            ) : null}
          </SurfaceCard>
        </aside>
      </div>
    </TronShell>
  );
}
