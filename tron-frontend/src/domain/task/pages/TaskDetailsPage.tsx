import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  Link,
  useSearchParams,
} from "react-router";
import { createPortal } from "react-dom";
import {
  Edit3,
  Maximize2,
  X,
  Plus,
  Save,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  Field,
  InspectorPanel,
  MarkdownContent,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  useConsoleShellConfig,
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
import { TaskAppSelectionField } from "@/domain/task/components";
import type {
  TaskPriority,
  TaskSourceType,
  TaskStatus,
  TaskType,
  UpdateTaskPayload,
} from "@/domain/task/model";
import {
  inferTaskRole,
  statusLabel,
} from "@/domain/task/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function ProgressBar({ value }: { value: number }) {
  return (
    <div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
      <div className="h-full rounded-full bg-[var(--color-primary)]" style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
}

function toggleCode(codes: string[], code: string) {
  return codes.includes(code) ? codes.filter((item) => item !== code) : [...codes, code];
}

function splitCodes(value: string) {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
}

function sameCodes(left: string[], right: string[]) {
  return left.length === right.length && left.every((item, index) => item === right[index]);
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
    <div className="min-h-6 py-1 text-sm leading-6 text-[var(--color-on-surface)] whitespace-pre-wrap">
      {children || <span className="text-[var(--color-on-surface-variant)]">None</span>}
    </div>
  );
}

export function TaskDetailsPage() {
  const [searchParams] = useSearchParams();
  const taskCode = searchParams.get("taskCode") || "TSK-SEED-FEATURE";
  const { task, setTask, refresh, loading } = useTaskDetail(taskCode);
  const options = useTaskOptions();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const { appOptions, effectiveUsers, loading: workspaceOptionsLoading } = useTaskWorkspaceOptions(task.workspaceCode);
  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description ?? "");
  const [taskType, setTaskType] = useState<TaskType>(task.taskType);
  const [sourceType, setSourceType] = useState<TaskSourceType>(task.sourceType);
  const [sourceUrl, setSourceUrl] = useState(task.sourceUrl ?? "");
  const [priority, setPriority] = useState<TaskPriority>(task.priority);
  const [status, setStatus] = useState<TaskStatus>(task.status);
  const [progress, setProgress] = useState(String(task.progress));
  const [agentCodesInput, setAgentCodesInput] = useState(task.agentCodes.join(", "));
  const [selectedAppCodes, setSelectedAppCodes] = useState(task.appCodes);
  const [assigneeUserCodes, setAssigneeUserCodes] = useState(task.assigneeUserCodes);
  const [childTitle, setChildTitle] = useState("Follow-up subtask");
  const [descriptionFullscreenOpen, setDescriptionFullscreenOpen] = useState(false);
  const [editing, setEditing] = useState(false);
  const skipNextTaskSync = useRef(false);
  const taskRef = useRef(task);
  const statusRef = useRef(status);
  const progressRef = useRef(progress);
  taskRef.current = task;
  statusRef.current = status;
  progressRef.current = progress;
  const archived = status === "Archived" || task.status === "Archived" || Boolean(task.archivedAt);
  const taskRole = inferTaskRole(task.taskType, task.parentTaskCode);
  const availableStatuses = options.statusRules[taskRole];
  const availableAppCodeSet = useMemo(() => new Set(appOptions.map((app) => app.appCode)), [appOptions]);
  const legacyAppCodes = selectedAppCodes.filter((code) => !availableAppCodeSet.has(code));

  useEffect(() => {
    if (skipNextTaskSync.current) {
      skipNextTaskSync.current = false;
      return;
    }
    setTitle(task.title);
    setDescription(task.description ?? "");
    setTaskType(task.taskType);
    setSourceType(task.sourceType);
    setSourceUrl(task.sourceUrl ?? "");
    setPriority(task.priority);
    setStatus(task.status);
    setProgress(String(task.progress));
    setAgentCodesInput(task.agentCodes.join(", "));
    setSelectedAppCodes(task.appCodes);
    setAssigneeUserCodes(task.assigneeUserCodes);
    setDescriptionFullscreenOpen(false);
    setEditing(false);
  }, [task]);

  useEffect(() => {
    if (!descriptionFullscreenOpen) return;

    const originalOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setDescriptionFullscreenOpen(false);
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [descriptionFullscreenOpen]);

  const effectiveUserByCode = new Map(effectiveUsers.map((user) => [user.userCode, user]));
  const assignedUsers = assigneeUserCodes.map((userCode) => effectiveUserByCode.get(userCode)?.displayName ?? userCode);
  const detailEditable = editing && !archived;
  const updatesEditable = !archived;
  const progressManagedBySubtasks = task.subtasks.length > 0;
  const nextAgentCodes = splitCodes(agentCodesInput);
  const nextAppCodes = Array.from(new Set(selectedAppCodes.map((code) => code.trim()).filter(Boolean)));
  const detailDirty = title !== task.title
    || description !== (task.description ?? "")
    || taskType !== task.taskType
    || sourceType !== task.sourceType
    || sourceUrl !== (task.sourceUrl ?? "");

  const rightPanel = useMemo(() => (
    <InspectorPanel title="Task Metadata">
      <SurfaceCard className="space-y-5 p-2">
        <div>
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
        </div>

        <div>
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
            {!archived ? (
              <>
                <div className="mt-4">
                  <Field label="New Subtask Title">
                    <TextInput aria-label="New subtask title" value={childTitle} onChange={(event) => setChildTitle(event.target.value)} />
                  </Field>
                </div>
                <Button className="mt-3 w-full" disabled={!childTitle.trim()} variant="secondary" onClick={addSubtask}><Plus size={14} aria-hidden="true" />Add Subtask</Button>
              </>
            ) : null}
        </div>
      </SurfaceCard>
    </InspectorPanel>
  ), [
    archived,
    assigneeUserCodes,
    availableAppCodeSet,
    childTitle,
    dataSource,
    selectedAppCodes,
    task,
  ]);

  useConsoleShellConfig({ rightPanel });
  const breadcrumbs = [
    { label: "Tasks", href: "/tasks" },
    ...(task.parent ? [{ label: task.parent.taskCode, href: taskDetailHref(task.parent.taskCode) }] : []),
    { label: task.taskCode },
  ];

  function resetDetailFields() {
    setTitle(task.title);
    setDescription(task.description ?? "");
    setTaskType(task.taskType);
    setSourceType(task.sourceType);
    setSourceUrl(task.sourceUrl ?? "");
    setDescriptionFullscreenOpen(false);
    setEditing(false);
  }

  async function saveDetailFields() {
    if (archived) {
      toast.warning("Restore archived task before editing");
      return;
    }
    try {
      const payload = {
        description,
        sourceType,
        sourceUrl,
        taskType,
        title,
      };
      const updated = dataSource === "api" ? await updateTask(task.taskCode, payload) : updateMockTask(task.taskCode, payload);
      setTask(updated);
      setDescriptionFullscreenOpen(false);
      setEditing(false);
      toast.success(`Task ${updated.taskCode} details saved`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Task details could not be updated");
    }
  }

  async function saveUpdateFields(overrides: Partial<{
    agentCodes: string[];
    appCodes: string[];
    assigneeUserCodes: string[];
    priority: TaskPriority;
    progress: number;
    status: TaskStatus;
  }> = {}, options: { onError?: () => void } = {}) {

    const draft = {
      agentCodes: nextAgentCodes,
      appCodes: nextAppCodes,
      assigneeUserCodes,
      priority,
      progress: progressValue(progress),
      status,
      ...overrides,
    };
    const hasChanges = draft.priority !== task.priority
      || draft.status !== task.status
      || draft.progress !== task.progress
      || !sameCodes(draft.agentCodes, task.agentCodes)
      || !sameCodes(draft.appCodes, task.appCodes)
      || !sameCodes(draft.assigneeUserCodes, task.assigneeUserCodes);

    if (!hasChanges) return;

    try {
      if (archived) return;

      const appCodesChanged = !sameCodes(draft.appCodes, task.appCodes);
      if (appCodesChanged && draft.appCodes.some((code) => !availableAppCodeSet.has(code))) {
        options.onError?.();
        toast.warning("Remove legacy app codes before changing app selection");
        return;
      }

      const payload: UpdateTaskPayload = {};
      if (!sameCodes(draft.agentCodes, task.agentCodes)) payload.agentCodes = draft.agentCodes;
      if (appCodesChanged) payload.appCodes = draft.appCodes;
      if (!sameCodes(draft.assigneeUserCodes, task.assigneeUserCodes)) payload.assigneeUserCodes = draft.assigneeUserCodes;
      if (draft.priority !== task.priority) payload.priority = draft.priority;

      let updated = task;
      if (Object.keys(payload).length > 0) {
        updated = dataSource === "api" ? await updateTask(task.taskCode, payload) : updateMockTask(task.taskCode, payload);
      }
      if (draft.status !== task.status || draft.progress !== task.progress) {
        updated = dataSource === "api"
          ? await updateTaskStatus(task.taskCode, { progress: draft.progress, status: draft.status })
          : updateMockTaskStatus(task.taskCode, { progress: draft.progress, status: draft.status });
      }
      skipNextTaskSync.current = true;
      setTask(updated);
      toast.success(`Task ${updated.taskCode} updates saved`);
    } catch (error) {
      options.onError?.();
      toast.error(error instanceof Error ? error.message : "Updates could not be saved");
    }
  }

  async function handleStatusChange(nextStatus: TaskStatus) {
    const currentTask = taskRef.current;
    const currentProgress = progressRef.current;
    const previousStatus = statusRef.current;
    const archivedNow = previousStatus === "Archived" || currentTask.status === "Archived" || Boolean(currentTask.archivedAt);
    setStatus(nextStatus);
    statusRef.current = nextStatus;
    if (nextStatus === previousStatus) return;

    try {
      if (nextStatus === "Archived" && !archivedNow) {
        const lifecycle = dataSource === "api" ? await archiveTask(currentTask.taskCode) : archiveMockTask(currentTask.taskCode);
        skipNextTaskSync.current = true;
        setTask({
          ...currentTask,
          archivedAt: lifecycle.archivedAt,
          previousStatus: lifecycle.previousStatus,
          status: lifecycle.status,
        });
        setStatus(lifecycle.status);
        statusRef.current = lifecycle.status;
        toast.success(`Task ${currentTask.taskCode} archived`);
        return;
      }

      if (archivedNow && nextStatus !== "Archived") {
        await (dataSource === "api" ? restoreTask(currentTask.taskCode) : restoreMockTask(currentTask.taskCode));
        const updated = dataSource === "api"
          ? await updateTaskStatus(currentTask.taskCode, { progress: progressValue(currentProgress), status: nextStatus })
          : updateMockTaskStatus(currentTask.taskCode, { progress: progressValue(currentProgress), status: nextStatus });
        skipNextTaskSync.current = true;
        setTask(updated);
        setStatus(updated.status);
        statusRef.current = updated.status;
        setProgress(String(updated.progress));
        progressRef.current = String(updated.progress);
        toast.success(`Task ${currentTask.taskCode} restored`);
        return;
      }

      const updated = dataSource === "api"
        ? await updateTaskStatus(currentTask.taskCode, { progress: progressValue(currentProgress), status: nextStatus })
        : updateMockTaskStatus(currentTask.taskCode, { progress: progressValue(currentProgress), status: nextStatus });
      skipNextTaskSync.current = true;
      setTask(updated);
      setStatus(updated.status);
      statusRef.current = updated.status;
      setProgress(String(updated.progress));
      progressRef.current = String(updated.progress);
      toast.success(`Task ${updated.taskCode} updates saved`);
    } catch (error) {
      setStatus(previousStatus);
      statusRef.current = previousStatus;
      toast.error(error instanceof Error ? error.message : "Updates could not be saved");
    }
  }

  function handlePriorityChange(nextPriority: TaskPriority) {
    setPriority(nextPriority);
    void saveUpdateFields({ priority: nextPriority });
  }

  function handleProgressBlur() {
    void saveUpdateFields({ progress: progressValue(progress) });
  }

  function handleAgentCodesBlur() {
    void saveUpdateFields({ agentCodes: splitCodes(agentCodesInput) });
  }

  function handleAppToggle(appCode: string) {
    const nextCodes = toggleCode(selectedAppCodes, appCode);
    setSelectedAppCodes(nextCodes);
    void saveUpdateFields({ appCodes: nextCodes }, {
      onError: () => setSelectedAppCodes(taskRef.current.appCodes),
    });
  }

  function handleLegacyAppRemove(appCode: string) {
    const nextCodes = selectedAppCodes.filter((code) => code !== appCode);
    setSelectedAppCodes(nextCodes);
    void saveUpdateFields({ appCodes: nextCodes }, {
      onError: () => setSelectedAppCodes(taskRef.current.appCodes),
    });
  }

  function handleAssigneeToggle(userCode: string) {
    const nextCodes = toggleCode(assigneeUserCodes, userCode);
    setAssigneeUserCodes(nextCodes);
    void saveUpdateFields({ assigneeUserCodes: nextCodes });
  }

  async function addSubtask() {
    if (archived) {
      toast.warning("Restore archived task before adding subtasks");
      return;
    }
    try {
      const payload = {
        agentCodes: [],
        appCodes: selectedAppCodes.filter((code) => availableAppCodeSet.has(code)),
        assigneeUserCodes: assigneeUserCodes.slice(0, 1),
        description: `Subtask for ${task.taskCode}`,
        parentTaskCode: task.taskCode,
        priority: "Low" as TaskPriority,
        sourceType: "Internal" as TaskSourceType,
        sourceUrl: "",
        taskType: task.taskType,
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

  return (
    <>
      <PageHeader
        breadcrumbs={breadcrumbs}
        title="Task Details"
        actions={
          <>
            <Button className="bg-black text-white hover:bg-neutral-800" variant="primary" onClick={() => {
              toast.info("Task execution is not available in v1");
            }}>Execute</Button>
            {detailEditable ? (
              <>
                <Button variant="secondary" onClick={resetDetailFields}>Cancel</Button>
                <Button disabled={!detailDirty} onClick={() => void saveDetailFields()}><Save size={15} aria-hidden="true" />Save</Button>
              </>
            ) : !archived ? (
              <Button variant="secondary" onClick={() => setEditing(true)}><Edit3 size={15} aria-hidden="true" />Edit Details</Button>
            ) : null}
          </>
        }
      />
      <div className="space-y-5">
          <SurfaceCard>
            <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Task Attributes</h2>
              </div>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Task Status">
                <SelectInput aria-label="Task detail status" value={status} onChange={(event) => {
                  void handleStatusChange(event.target.value as TaskStatus);
                }}>
                  {availableStatuses.map((item) => <option key={item} value={item}>{statusLabel(item)}</option>)}
                </SelectInput>
              </Field>
              <Field label="Priority">
                {updatesEditable ? (
                  <SelectInput aria-label="Task detail priority" value={priority} onChange={(event) => handlePriorityChange(event.target.value as TaskPriority)}>
                    {options.priorities.map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{task.priority}</ReadValue>
                )}
              </Field>
              <Field label="Progress">
                {updatesEditable ? (
                  <TextInput
                    aria-label="Task detail progress"
                    disabled={progressManagedBySubtasks}
                    max="100"
                    min="0"
                    type="number"
                    value={progressManagedBySubtasks ? String(task.progress) : progress}
                    onChange={(event) => setProgress(event.target.value)}
                    onBlur={handleProgressBlur}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") event.currentTarget.blur();
                    }}
                  />
                ) : (
                  <ReadValue>{task.progress}%</ReadValue>
                )}
              </Field>
              <Field label="Agent Codes">
                {updatesEditable ? (
                  <TextInput
                    aria-label="Task detail agent codes"
                    placeholder="agent-reviewer, agent-builder"
                    value={agentCodesInput}
                    onChange={(event) => setAgentCodesInput(event.target.value)}
                    onBlur={handleAgentCodesBlur}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") event.currentTarget.blur();
                    }}
                  />
                ) : (
                  <ReadValue>{task.agentCodes.join(", ") || "None"}</ReadValue>
                )}
              </Field>
              <div className="md:col-span-2">
                <Field label="Apps" hint="Only active apps from this workspace can be newly assigned.">
                  <TaskAppSelectionField
                    availableApps={appOptions}
                    disabled={!updatesEditable}
                    emptyMessage="No active apps are available for this workspace."
                    legacyAppCodes={legacyAppCodes}
                    loading={workspaceOptionsLoading}
                    selectedAppCodes={selectedAppCodes}
                    onRemoveLegacy={updatesEditable ? handleLegacyAppRemove : undefined}
                    onToggle={updatesEditable ? handleAppToggle : undefined}
                  />
                </Field>
              </div>
            </div>
            <SurfaceCard className="mt-4 bg-[var(--color-surface-container-lowest)]">
              <h3 className="mb-3 text-sm font-semibold">Assigned Users</h3>
              {updatesEditable ? (
                <div className="grid gap-2 md:grid-cols-2">
                  {effectiveUsers.map((user) => (
                    <label className="flex items-start gap-3 rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={user.userCode}>
                      <input
                        aria-label={`Assign ${user.displayName}`}
                        checked={assigneeUserCodes.includes(user.userCode)}
                        className="mt-1"
                        type="checkbox"
                        onChange={() => handleAssigneeToggle(user.userCode)}
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
          <SurfaceCard>
            <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Task Definition</h2>
                <p className="mt-1 font-mono text-sm text-[var(--color-on-surface-variant)]">{task.taskCode}</p>
              </div>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Title">
                {detailEditable ? (
                  <TextInput aria-label="Task detail title" value={title} onChange={(event) => setTitle(event.target.value)} />
                ) : (
                  <ReadValue>{task.title}</ReadValue>
                )}
              </Field>
              <Field label="Task Type">
                {detailEditable ? (
                  <SelectInput aria-label="Task detail type" value={taskType} onChange={(event) => setTaskType(event.target.value as TaskType)}>
                    {options.taskTypes.map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{task.taskType}</ReadValue>
                )}
              </Field>
              <Field label="Source Type">
                {detailEditable ? (
                  <SelectInput aria-label="Task detail source type" value={sourceType} onChange={(event) => setSourceType(event.target.value as TaskSourceType)}>
                    {options.sourceTypes.map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                ) : (
                  <ReadValue>{task.sourceType}</ReadValue>
                )}
              </Field>
              <Field label="Source URL">
                {detailEditable ? (
                  <TextInput aria-label="Task detail source URL" placeholder="Optional GitHub/GitLab URL" value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
                ) : (
                  <ReadValue>{task.sourceUrl || "None"}</ReadValue>
                )}
              </Field>
            </div>
            <div className="mt-4 grid gap-1.5">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Description</span>
                <Button
                  size="sm"
                  type="button"
                  variant="ghost"
                  aria-label="Open description fullscreen"
                  onClick={() => setDescriptionFullscreenOpen(true)}
                >
                  <Maximize2 size={14} aria-hidden="true" />
                  Fullscreen
                </Button>
              </div>
              {detailEditable ? (
                <TextArea aria-label="Task detail description" className="min-h-36 w-full" value={description} onChange={(event) => setDescription(event.target.value)} />
              ) : (
                <MarkdownContent ariaLabel="Task description markdown" className="max-h-80 overflow-y-auto pr-2" content={task.description} />
              )}
            </div>
          </SurfaceCard>
      </div>
      {descriptionFullscreenOpen ? createPortal(
        <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={detailEditable ? "Description fullscreen editor" : "Description fullscreen preview"}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Description</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {detailEditable ? "Edit the task description in a larger workspace." : "Preview the rendered markdown for this task description."}
                </p>
              </div>
              <Button
                size="icon"
                type="button"
                variant="ghost"
                aria-label="Close description fullscreen"
                onClick={() => setDescriptionFullscreenOpen(false)}
              >
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              {detailEditable ? (
                <TextArea
                  aria-label="Task detail description fullscreen"
                  className="h-full min-h-0 w-full resize-none"
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                />
              ) : (
                <div className="h-full overflow-y-auto pr-1">
                  <MarkdownContent ariaLabel="Task description markdown fullscreen" className="max-w-none" content={task.description} />
                </div>
              )}
            </div>
          </div>
        </div>,
        document.body,
      ) : null}
    </>
  );
}
