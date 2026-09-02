import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  Link,
  useNavigate,
  useSearchParams,
} from "react-router";
import { useTranslation } from "react-i18next";
import { createPortal } from "react-dom";
import {
  Edit3,
  Maximize2,
  Play,
  X,
  Plus,
  Save,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Field,
  InspectorPanel,
  MarkdownContent,
  MultiSelectDropdown,
  OptionSelect,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  useTabPanelConfig,
  useToast,
} from "@/shared";

import {
  archiveTask,
  createTask,
  executeTask,
  restoreTask,
  updateTask,
  updateTaskStatus,
} from "@/domain/task/api";
import {
  archiveMockTask,
  createMockTask,
  executeMockTask,
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
import { fetchWorkspaceSystemAgents } from "@/domain/agent/api";
import { listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import { fetchWorkspaceSelectOptions } from "@/domain/model/api/modelApi";
import { getWorkspaceSelectOptions } from "@/domain/model/mock/data";
import {
  automationModelCodeFromOption,
  automationModelOptionsForAgent,
  availableAutomationAgents,
  mainAutomationAgentOption,
  optionCodeForAutomationModel,
} from "@/domain/automation/pages/automationExecutionOptions";
import type {
  AutomationAgentOption,
  AutomationModelOption,
} from "@/domain/automation/pages/automationExecutionOptions";
import { SelectItem } from "@/components/ui/select";

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

function ReadValue({ children, noneLabel = "None" }: { children: ReactNode; noneLabel?: string }) {
  return (
    <div className="min-h-6 min-w-0 max-w-full py-1 text-sm leading-6 text-[var(--color-on-surface)] whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
      {children || <span className="text-[var(--color-on-surface-variant)]">{noneLabel}</span>}
    </div>
  );
}

export function TaskDetailsPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = useCallback((key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`tasks.${key}`, { defaultValue, ...options })
  ), [t]);
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
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
  const [agentOptions, setAgentOptions] = useState<AutomationAgentOption[]>(() => [mainAutomationAgentOption]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(task.agentCode || mainAutomationAgentOption.agentCode);
  const [selectedModelOptionCode, setSelectedModelOptionCode] = useState(optionCodeForAutomationModel(task.agentCode || mainAutomationAgentOption.agentCode, task.modelCode ?? null));
  const [workspaceModelOptions, setWorkspaceModelOptions] = useState<ReturnType<typeof getWorkspaceSelectOptions>>(() => []);
  const [executionOptionsLoading, setExecutionOptionsLoading] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [agentCodesInput, setAgentCodesInput] = useState(task.agentCodes.join(", "));
  const [selectedAppCodes, setSelectedAppCodes] = useState(task.appCodes);
  const [assigneeUserCodes, setAssigneeUserCodes] = useState(task.assigneeUserCodes);
  const [childTitle, setChildTitle] = useState("Follow-up subtask");
  const [descriptionFullscreenOpen, setDescriptionFullscreenOpen] = useState(false);
  const [executionResultFullscreenOpen, setExecutionResultFullscreenOpen] = useState(false);
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
  const configuredAgentOption = useMemo<AutomationAgentOption>(() => {
    const configuredAgentCode = selectedAgentCode || mainAutomationAgentOption.agentCode;
    const matched = agentOptions.find((agent) => agent.agentCode === configuredAgentCode);
    if (matched) return matched;
    if (configuredAgentCode === mainAutomationAgentOption.agentCode) return mainAutomationAgentOption;
    return {
      agentCode: configuredAgentCode,
      description: "Configured task execution agent.",
      models: [],
      name: configuredAgentCode,
    };
  }, [agentOptions, selectedAgentCode]);
  const visibleAgentOptions = useMemo(
    () => agentOptions.some((agent) => agent.agentCode === configuredAgentOption.agentCode)
      ? agentOptions
      : [configuredAgentOption, ...agentOptions],
    [agentOptions, configuredAgentOption],
  );
  const executionAgentSelectOptions = useMemo(
    () => visibleAgentOptions.map((agent) => ({
      label: `${agent.name} (${agent.agentCode})`,
      value: agent.agentCode,
    })),
    [visibleAgentOptions],
  );
  const modelOptions = useMemo(
    () => automationModelOptionsForAgent(workspaceModelOptions, configuredAgentOption),
    [configuredAgentOption, workspaceModelOptions],
  );
  const selectedModelCode = useMemo(() => {
    const resolvedModelCode = automationModelCodeFromOption(modelOptions, selectedModelOptionCode);
    if (resolvedModelCode) return resolvedModelCode;
    const configuredOptionCode = optionCodeForAutomationModel(selectedAgentCode, task.modelCode ?? null);
    if (task.modelCode && selectedModelOptionCode === configuredOptionCode && !modelOptions.some((option) => option.optionCode === selectedModelOptionCode)) {
      return task.modelCode;
    }
    return null;
  }, [modelOptions, selectedAgentCode, selectedModelOptionCode, task.modelCode]);
  const visibleModelOptions = useMemo<AutomationModelOption[]>(() => {
    if (!selectedModelOptionCode || modelOptions.some((option) => option.optionCode === selectedModelOptionCode)) {
      return modelOptions;
    }
    if (!task.modelCode) return modelOptions;
    return [{
      label: task.modelCode,
      modelCode: task.modelCode,
      optionCode: selectedModelOptionCode,
    }, ...modelOptions];
  }, [modelOptions, selectedModelOptionCode, task.modelCode]);
  const executionModelSelectOptions = useMemo(
    () => visibleModelOptions.map((model) => ({
      label: model.label,
      value: model.optionCode,
    })),
    [visibleModelOptions],
  );

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
    setSelectedAgentCode(task.agentCode || mainAutomationAgentOption.agentCode);
    setSelectedModelOptionCode(optionCodeForAutomationModel(task.agentCode || mainAutomationAgentOption.agentCode, task.modelCode ?? null));
    setAgentCodesInput(task.agentCodes.join(", "));
    setSelectedAppCodes(task.appCodes);
    setAssigneeUserCodes(task.assigneeUserCodes);
    setDescriptionFullscreenOpen(false);
    setExecutionResultFullscreenOpen(false);
    setEditing(false);
  }, [task]);

  useEffect(() => {
    if (!task.workspaceCode) {
      setAgentOptions([mainAutomationAgentOption]);
      setWorkspaceModelOptions([]);
      return;
    }

    let active = true;
    setExecutionOptionsLoading(true);
    const agentsPromise = getServiceDataSource("agent") === "api"
      ? fetchWorkspaceSystemAgents(task.workspaceCode)
      : Promise.resolve(listWorkspaceSystemAgentsMock(task.workspaceCode));
    const modelsPromise = getServiceDataSource("model") === "api"
      ? fetchWorkspaceSelectOptions(task.workspaceCode)
      : Promise.resolve(getWorkspaceSelectOptions(task.workspaceCode));

    Promise.all([agentsPromise, modelsPromise])
      .then(([agents, models]) => {
        if (!active) return;
        setAgentOptions(availableAutomationAgents(agents));
        setWorkspaceModelOptions(models);
      })
      .catch((error) => {
        if (!active) return;
        setAgentOptions([mainAutomationAgentOption]);
        setWorkspaceModelOptions([]);
        toast.error(error instanceof Error ? error.message : pageT("errors.executionOptions", "Execution options could not be loaded"));
      })
      .finally(() => {
        if (active) setExecutionOptionsLoading(false);
      });

    return () => {
      active = false;
    };
  }, [pageT, task.workspaceCode, toast]);

  useEffect(() => {
    if (visibleAgentOptions.some((agent) => agent.agentCode === selectedAgentCode)) return;
    setSelectedAgentCode(mainAutomationAgentOption.agentCode);
  }, [selectedAgentCode, visibleAgentOptions]);

  useEffect(() => {
    if (!visibleModelOptions.length) {
      setSelectedModelOptionCode("");
      return;
    }
    if (visibleModelOptions.some((option) => option.optionCode === selectedModelOptionCode)) return;
    setSelectedModelOptionCode(visibleModelOptions[0].optionCode);
  }, [selectedModelOptionCode, visibleModelOptions]);

  useEffect(() => {
    if (!descriptionFullscreenOpen && !executionResultFullscreenOpen) return;

    const originalOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setDescriptionFullscreenOpen(false);
      if (event.key === "Escape") setExecutionResultFullscreenOpen(false);
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [descriptionFullscreenOpen, executionResultFullscreenOpen]);

  const effectiveUserByCode = new Map(effectiveUsers.map((user) => [user.userCode, user]));
  const userOptions = effectiveUsers.map((user) => ({
    description: user.userCode,
    label: user.displayName,
    value: user.userCode,
  }));
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
    || sourceUrl !== (task.sourceUrl ?? "")
    || selectedAgentCode !== (task.agentCode || mainAutomationAgentOption.agentCode)
    || selectedModelCode !== task.modelCode;

  const rightPanel = useMemo(() => (
    <InspectorPanel title={pageT("detail.metadata", "Task Metadata")}>
      <SurfaceCard className="space-y-5 p-2">
        <div>
          <h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{pageT("detail.metadata", "Task Metadata")}</h3>
          <div className="space-y-3 text-sm">
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.workspace", "Workspace")}</span><span className="font-medium">{task.workspaceCode}</span></div>
            <div className="flex justify-between gap-3">
              <span className="text-[var(--color-on-surface-variant)]">{pageT("detail.parent", "Parent")}</span>
              {task.parent ? (
                <Link className="text-right font-medium text-[var(--color-primary)] hover:underline" to={taskDetailHref(task.parent.taskCode)}>
                  {task.parent.taskCode}
                </Link>
              ) : (
                <span className="font-medium">{pageT("detail.none", "None")}</span>
              )}
            </div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.created", "Created")}</span><span className="font-medium">{task.createdAt}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.updated", "Updated")}</span><span className="font-medium">{task.updatedAt}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.archivedAt", "Archived At")}</span><span className="font-medium">{task.archivedAt ?? pageT("detail.notArchived", "Not archived")}</span></div>
          </div>
        </div>

        <div>
          <h3 className="mb-3 border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{pageT("detail.subtasks", "Subtasks")}</h3>
          <div className="space-y-2">
            {task.subtasks.map((item) => (
              <Link className="block rounded-lg border border-[var(--color-outline-variant)] p-3 hover:border-[var(--color-primary)]" key={item.taskCode} to={taskDetailHref(item.taskCode)}>
                <div className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{item.taskCode}</div>
                <div className="mt-1 text-sm font-medium">{item.title}</div>
                <div className="mt-2 flex items-center justify-between gap-3">
                  <span className="text-xs text-[var(--color-on-surface-variant)]">{pageT(`statuses.${item.status}`, statusLabel(item.status))}</span>
                  <span className="text-xs text-[var(--color-on-surface-variant)]">{item.progress}%</span>
                </div>
                <div className="mt-2"><ProgressBar value={item.progress} /></div>
              </Link>
            ))}
            {!task.subtasks.length ? <p className="text-sm text-[var(--color-on-surface-variant)]">{pageT("detail.noSubtasks", "No active subtasks.")}</p> : null}
          </div>
          {!archived ? (
            <>
              <div className="mt-4">
                <Field label={pageT("detail.newSubtaskTitle", "New Subtask Title")}>
                  <TextInput aria-label={pageT("detail.newSubtaskTitle", "New subtask title")} value={childTitle} onChange={(event) => setChildTitle(event.target.value)} />
                </Field>
              </div>
              <Button className="mt-3 w-full" disabled={!childTitle.trim()} variant="secondary" onClick={addSubtask}><Plus size={14} aria-hidden="true" />{pageT("detail.addSubtask", "Add Subtask")}</Button>
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
    pageT,
    selectedAppCodes,
    task,
  ]);

  useTabPanelConfig({ rightPanel });
  const breadcrumbs = [
    { label: pageT("title", "Tasks"), href: "/tasks" },
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
    setExecutionResultFullscreenOpen(false);
    setEditing(false);
  }

  async function saveDetailFields() {
    if (archived) {
      toast.warning(pageT("warnings.restoreBeforeEditing", "Restore archived task before editing"));
      return;
    }
    try {
      const payload = {
        agentCode: selectedAgentCode,
        description,
        modelCode: selectedModelCode,
        sourceType,
        sourceUrl,
        taskType,
        title,
      };
      const updated = dataSource === "api" ? await updateTask(task.taskCode, payload) : updateMockTask(task.taskCode, payload);
      setTask(updated);
      setDescriptionFullscreenOpen(false);
      setExecutionResultFullscreenOpen(false);
      setEditing(false);
      toast.success(pageT("messages.detailsSaved", "Task {{code}} details saved", { code: updated.taskCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.update", "Task details could not be updated"));
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
        toast.warning(pageT("warnings.removeLegacyAppCodes", "Remove legacy app codes before changing app selection"));
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
      toast.success(pageT("messages.updatesSaved", "Task {{code}} updates saved", { code: updated.taskCode }));
    } catch (error) {
      options.onError?.();
      toast.error(error instanceof Error ? error.message : pageT("errors.updates", "Updates could not be saved"));
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
        toast.success(pageT("messages.archived", "Task {{code}} archived", { code: currentTask.taskCode }));
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
        toast.success(pageT("messages.restored", "Task {{code}} restored", { code: currentTask.taskCode }));
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
      toast.success(pageT("messages.updatesSaved", "Task {{code}} updates saved", { code: updated.taskCode }));
    } catch (error) {
      setStatus(previousStatus);
      statusRef.current = previousStatus;
      toast.error(error instanceof Error ? error.message : pageT("errors.updates", "Updates could not be saved"));
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
      toast.warning(pageT("warnings.restoreBeforeSubtasks", "Restore archived task before adding subtasks"));
      return;
    }
    try {
      const payload = {
        agentCodes: [],
        agentCode: task.agentCode,
        appCodes: selectedAppCodes.filter((code) => availableAppCodeSet.has(code)),
        assigneeUserCodes: assigneeUserCodes.slice(0, 1),
        description: `Subtask for ${task.taskCode}`,
        modelCode: task.modelCode,
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
      toast.success(pageT("messages.subtaskCreated", "Subtask {{code}} created", { code: child.taskCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.subtask", "Subtask could not be created"));
    }
  }

  async function handleExecute() {
    if (executing || archived) return;
    setExecuting(true);
    try {
      const result = dataSource === "api" ? await executeTask(task.taskCode) : executeMockTask(task.taskCode);
      toast.success(pageT("messages.executionStarted", "Task {{code}} execution started", { code: result.taskCode }), { title: result.runCode });
      navigate(`/chats?sessionCode=${encodeURIComponent(result.sessionCode)}&sourceTab=chat`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:tasks.detail.errors.execution", "Task execution failed"));
    } finally {
      setExecuting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={breadcrumbs}
        title={t("pages:tasks.detail.title", "Task Details")}
        actions={
          <>
            <Button disabled={executing || archived} size="lg" onClick={() => void handleExecute()}>
              {executing ? t("pages:tasks.detail.actions.executing", "Executing...") : t("pages:tasks.detail.actions.execute", "Execute")}
            </Button>
            {detailEditable ? (
              <>
                <Button size="lg" variant="outline" onClick={resetDetailFields}>{t("pages:tasks.actions.cancel", "Cancel")}</Button>
                <Button size="lg" disabled={!detailDirty} onClick={() => void saveDetailFields()}>{t("pages:tasks.actions.save", "Save")}</Button>
              </>
            ) : !archived ? (
              <Button size="lg" variant="outline" onClick={() => setEditing(true)}>{t("pages:tasks.detail.actions.edit", "Edit")}</Button>
            ) : null}
          </>
        }
      />
      <div className="space-y-5">
        <SurfaceCard>
          <div className="mb-5">
            <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{t("pages:tasks.detail.title", "Task Details")}</h2>
            <p className="mt-1 font-mono text-sm text-[var(--color-on-surface-variant)]">{task.taskCode}</p>
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={t("pages:tasks.detail.status", "Task Status")}>
              <SelectInput aria-label={t("pages:tasks.detail.statusAria", "Task detail status")} value={status} onValueChange={(event) => {
                void handleStatusChange(event as TaskStatus);
              }}>
                {availableStatuses.map((item) => <SelectItem key={item} value={item}>{pageT(`statuses.${item}`, statusLabel(item))}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={t("pages:tasks.fields.priority", "Priority")}>
              {updatesEditable ? (
                <SelectInput aria-label={t("pages:tasks.detail.priorityAria", "Task detail priority")} value={priority} onValueChange={(event) => handlePriorityChange(event as TaskPriority)}>
                  {options.priorities.map((item) => <SelectItem key={item} value={item}>{pageT(`priorities.${item}`, item)}</SelectItem>)}
                </SelectInput>
              ) : (
                <ReadValue>{pageT(`priorities.${task.priority}`, task.priority)}</ReadValue>
              )}
            </Field>
            <Field label={t("pages:tasks.detail.progress", "Progress")}>
              {updatesEditable ? (
                <TextInput
                  aria-label={t("pages:tasks.detail.progressAria", "Task detail progress")}
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
            <Field label={t("pages:tasks.fields.agentCodes", "Agent Codes")}>
              {updatesEditable ? (
                <TextInput
                  aria-label={t("pages:tasks.detail.agentCodesAria", "Task detail agent codes")}
                  placeholder={pageT("placeholders.agentCodes", "agent-reviewer, agent-builder")}
                  value={agentCodesInput}
                  onChange={(event) => setAgentCodesInput(event.target.value)}
                  onBlur={handleAgentCodesBlur}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") event.currentTarget.blur();
                  }}
                />
              ) : (
                <ReadValue noneLabel={pageT("detail.none", "None")}>{task.agentCodes.join(", ")}</ReadValue>
              )}
            </Field>
            <Field label={pageT("fields.executionAgent", "Execution Agent")}>
              {detailEditable ? (
                <OptionSelect
                  ariaLabel={pageT("aria.executionAgent", "Task execution agent")}
                  disabled={executionOptionsLoading}
                  loading={executionOptionsLoading}
                  loadingLabel={pageT("values.loadingAgents", "Loading agents...")}
                  options={executionAgentSelectOptions}
                  value={selectedAgentCode}
                  onChange={setSelectedAgentCode}
                />
              ) : (
                <ReadValue>{task.agentCode || "main-agent"}</ReadValue>
              )}
            </Field>
            <Field label={pageT("fields.executionModel", "Execution Model")}>
              {detailEditable ? (
                <OptionSelect
                  ariaLabel={pageT("aria.executionModel", "Task execution model")}
                  emptyLabel={pageT("values.noModels", "No executable models")}
                  disabled={executionOptionsLoading || !visibleModelOptions.length}
                  loading={executionOptionsLoading}
                  loadingLabel={pageT("values.loadingModels", "Loading models...")}
                  options={executionModelSelectOptions}
                  value={selectedModelOptionCode}
                  onChange={setSelectedModelOptionCode}
                />
              ) : (
                <ReadValue>{task.modelCode || "Workspace default text model"}</ReadValue>
              )}
            </Field>
            <Field label={pageT("fields.title", "Title")}>
              {detailEditable ? (
                <TextInput aria-label={pageT("detail.titleAria", "Task detail title")} value={title} onChange={(event) => setTitle(event.target.value)} />
              ) : (
                <ReadValue>{task.title}</ReadValue>
              )}
            </Field>
            <Field label={pageT("fields.taskType", "Task Type")}>
              {detailEditable ? (
                <SelectInput aria-label={pageT("detail.typeAria", "Task detail type")} value={taskType} onValueChange={(event) => setTaskType(event as TaskType)}>
                  {options.taskTypes.map((item) => <SelectItem key={item} value={item}>{pageT(`types.${item}`, item)}</SelectItem>)}
                </SelectInput>
              ) : (
                <ReadValue>{pageT(`types.${task.taskType}`, task.taskType)}</ReadValue>
              )}
            </Field>
            <Field label={pageT("fields.sourceType", "Source Type")}>
              {detailEditable ? (
                <SelectInput aria-label={pageT("detail.sourceTypeAria", "Task detail source type")} value={sourceType} onValueChange={(event) => setSourceType(event as TaskSourceType)}>
                  {options.sourceTypes.map((item) => <SelectItem key={item} value={item}>{pageT(`sourceTypes.${item}`, item)}</SelectItem>)}
                </SelectInput>
              ) : (
                <ReadValue>{pageT(`sourceTypes.${task.sourceType}`, task.sourceType)}</ReadValue>
              )}
            </Field>
            <Field label={pageT("fields.sourceUrl", "Source URL")}>
              {detailEditable ? (
                <TextInput aria-label={pageT("detail.sourceUrlAria", "Task detail source URL")} placeholder={pageT("placeholders.sourceUrl", "Optional GitHub/GitLab URL")} value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
              ) : (
                <ReadValue noneLabel={pageT("detail.none", "None")}>{task.sourceUrl}</ReadValue>
              )}
            </Field>
            <div className="md:col-span-2">
              <Field label={pageT("fields.apps", "Apps")} hint={pageT("hints.apps", "Only active apps from this workspace can be newly assigned.")}>
                <TaskAppSelectionField
                  availableApps={appOptions}
                  disabled={!updatesEditable}
                  emptyMessage={pageT("values.noActiveApps", "No active apps are available for this workspace.")}
                  legacyAppCodes={legacyAppCodes}
                  loading={workspaceOptionsLoading}
                  selectedAppCodes={selectedAppCodes}
                  onRemoveLegacy={updatesEditable ? handleLegacyAppRemove : undefined}
                  onToggle={updatesEditable ? handleAppToggle : undefined}
                />
              </Field>
            </div>
            <div className="md:col-span-2">
              <Field label={pageT("fields.assignedUsers", "Assigned Users")} hint={pageT("hints.assignedUsers", "Only effective users from the current workspace can be assigned.")}>
                {updatesEditable ? (
                  <MultiSelectDropdown
                    ariaLabel={pageT("aria.assignees", "Task assignees")}
                    emptyMessage={pageT("values.noUsers", "No effective users available for this workspace.")}
                    loading={workspaceOptionsLoading}
                    loadingMessage={pageT("values.loadingUsers", "Loading user options...")}
                    options={userOptions}
                    placeholder={pageT("placeholders.assignedUsers", "Select assigned users")}
                    selectedValues={assigneeUserCodes}
                    onToggle={handleAssigneeToggle}
                  />
                ) : (
                  <ReadValue noneLabel={pageT("detail.none", "None")}>{assignedUsers.join(", ")}</ReadValue>
                )}
              </Field>
            </div>
          </div>
          <div className="mt-4 grid gap-1.5">
            <div className="flex flex-wrap items-center justify-between gap-[12px]">
              <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("fields.description", "Description")}</span>
              <Button
                size="lg"
                type="button"
                variant="ghost"
                aria-label={pageT("aria.openDescription", "Open description fullscreen")}
                onClick={() => setDescriptionFullscreenOpen(true)}
              >
                <Maximize2 size={14} aria-hidden="true" />
              </Button>
            </div>
            {detailEditable ? (
              <TextArea aria-label={pageT("detail.descriptionAria", "Task detail description")} className="min-h-36 w-full" value={description} onChange={(event) => setDescription(event.target.value)} />
            ) : (
              <MarkdownContent ariaLabel={pageT("detail.descriptionMarkdownAria", "Task description markdown")} className="max-h-80 overflow-y-auto pr-2" content={task.description} />
            )}
          </div>
          <div className="mt-4 grid gap-1.5 border-t border-[var(--color-outline-variant)] pt-4">
            <div className="flex flex-wrap items-center justify-between gap-[12px]">
              <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("detail.executionResult", "Execution Result")}</span>
              <Button
                size="lg"
                type="button"
                variant="ghost"
                aria-label={pageT("detail.openExecutionResultFullscreen", "Open execution result fullscreen")}
                disabled={!task.executionResult}
                onClick={() => setExecutionResultFullscreenOpen(true)}
              >
                <Maximize2 size={14} aria-hidden="true" />
              </Button>
            </div>
            <MarkdownContent ariaLabel={pageT("detail.executionResultMarkdownAria", "Task execution result markdown")} className="max-h-80 overflow-y-auto pr-2" content={task.executionResult} />
          </div>
        </SurfaceCard>
      </div>
      {descriptionFullscreenOpen ? createPortal(
        <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={detailEditable ? pageT("detail.descriptionFullscreenEditorAria", "Description fullscreen editor") : pageT("detail.descriptionFullscreenPreviewAria", "Description fullscreen preview")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("fields.description", "Description")}</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {detailEditable ? pageT("detail.descriptionEditHint", "Edit the task description in a larger workspace.") : pageT("detail.descriptionPreviewHint", "Preview the rendered markdown for this task description.")}
                </p>
              </div>
              <Button
                size="icon"
                type="button"
                variant="ghost"
                aria-label={pageT("aria.closeDescription", "Close description fullscreen")}
                onClick={() => setDescriptionFullscreenOpen(false)}
              >
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              {detailEditable ? (
                <TextArea
                  aria-label={pageT("detail.descriptionFullscreenAria", "Task detail description fullscreen")}
                  className="h-full min-h-0 w-full resize-none"
                  value={description}
                  onChange={(event) => setDescription(event.target.value)}
                />
              ) : (
                <div className="h-full overflow-y-auto pr-1">
                  <MarkdownContent ariaLabel={pageT("detail.descriptionMarkdownFullscreenAria", "Task description markdown fullscreen")} className="max-w-none" content={task.description} />
                </div>
              )}
            </div>
          </div>
        </div>,
        document.body,
      ) : null}
      {executionResultFullscreenOpen ? createPortal(
        <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={pageT("detail.executionResultFullscreenAria", "Execution result fullscreen preview")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("detail.executionResult", "Execution Result")}</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{pageT("detail.executionResultHint", "Preview the final agent reply recorded for this task execution.")}</p>
              </div>
              <Button
                size="icon"
                type="button"
                variant="ghost"
                aria-label={pageT("detail.closeExecutionResultFullscreen", "Close execution result fullscreen")}
                onClick={() => setExecutionResultFullscreenOpen(false)}
              >
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              <div className="h-full overflow-y-auto pr-1">
                <MarkdownContent ariaLabel={pageT("detail.executionResultMarkdownFullscreenAria", "Task execution result markdown fullscreen")} className="max-w-none" content={task.executionResult} />
              </div>
            </div>
          </div>
        </div>,
        document.body,
      ) : null}
    </>
  );
}
