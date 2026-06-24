import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Maximize2,
  Save,
  X,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  Field,
  MultiSelectDropdown,
  OptionSelect,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import { createTask } from "@/domain/task/api";
import { createMockTask } from "@/domain/task/mock/data";
import {
  useTaskOptions,
  useTaskWorkspaceOptions,
} from "@/domain/task/hooks";
import { TaskAppSelectionField } from "@/domain/task/components";
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
} from "@/domain/automation/pages/automationExecutionOptions";
import type { AutomationAgentOption } from "@/domain/automation/pages/automationExecutionOptions";
import type {
  TaskPriority,
  TaskSourceType,
  TaskType,
} from "@/domain/task/model";

function toggleCode(codes: string[], code: string) {
  return codes.includes(code) ? codes.filter((item) => item !== code) : [...codes, code];
}

function splitCodes(value: string) {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
}

export function CreateTaskPage() {
  const options = useTaskOptions();
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [taskType, setTaskType] = useState<TaskType>("Feature");
  const [sourceType, setSourceType] = useState<TaskSourceType>("Internal");
  const [sourceUrl, setSourceUrl] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("Medium");
  const [agentOptions, setAgentOptions] = useState<AutomationAgentOption[]>(() => [mainAutomationAgentOption]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(mainAutomationAgentOption.agentCode);
  const [selectedModelOptionCode, setSelectedModelOptionCode] = useState("");
  const [workspaceModelOptions, setWorkspaceModelOptions] = useState<ReturnType<typeof getWorkspaceSelectOptions>>(() => []);
  const [executionOptionsLoading, setExecutionOptionsLoading] = useState(false);
  const [agentCodesInput, setAgentCodesInput] = useState("");
  const [selectedAppCodes, setSelectedAppCodes] = useState<string[]>([]);
  const [assigneeUserCodes, setAssigneeUserCodes] = useState<string[]>([]);
  const [descriptionFullscreenOpen, setDescriptionFullscreenOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const { appOptions, effectiveUsers, loading: workspaceOptionsLoading } = useTaskWorkspaceOptions(activeWorkspaceCode);
  const executionAgentSelectOptions = useMemo(
    () => agentOptions.map((agent) => ({
      label: `${agent.name} (${agent.agentCode})`,
      value: agent.agentCode,
    })),
    [agentOptions],
  );
  const userOptions = useMemo(
    () => effectiveUsers.map((user) => ({
      description: user.userCode,
      label: user.displayName,
      value: user.userCode,
    })),
    [effectiveUsers],
  );
  const selectedAgent = useMemo(
    () => agentOptions.find((agent) => agent.agentCode === selectedAgentCode) ?? mainAutomationAgentOption,
    [agentOptions, selectedAgentCode],
  );
  const modelOptions = useMemo(
    () => automationModelOptionsForAgent(workspaceModelOptions, selectedAgent),
    [selectedAgent, workspaceModelOptions],
  );
  const executionModelSelectOptions = useMemo(
    () => modelOptions.map((model) => ({
      label: model.label,
      value: model.optionCode,
    })),
    [modelOptions],
  );
  const selectedModelCode = useMemo(
    () => automationModelCodeFromOption(modelOptions, selectedModelOptionCode),
    [modelOptions, selectedModelOptionCode],
  );

  useEffect(() => {
    if (agentOptions.some((agent) => agent.agentCode === selectedAgentCode)) return;
    setSelectedAgentCode(mainAutomationAgentOption.agentCode);
  }, [agentOptions, selectedAgentCode]);

  useEffect(() => {
    if (!modelOptions.length) {
      setSelectedModelOptionCode("");
      return;
    }
    if (modelOptions.some((option) => option.optionCode === selectedModelOptionCode)) return;
    setSelectedModelOptionCode(modelOptions[0].optionCode);
  }, [modelOptions, selectedModelOptionCode]);

  useEffect(() => {
    if (!activeWorkspaceCode) {
      setAgentOptions([mainAutomationAgentOption]);
      setWorkspaceModelOptions([]);
      return;
    }

    let active = true;
    setExecutionOptionsLoading(true);
    const agentsPromise = getServiceDataSource("agent") === "api"
      ? fetchWorkspaceSystemAgents(activeWorkspaceCode)
      : Promise.resolve(listWorkspaceSystemAgentsMock(activeWorkspaceCode));
    const modelsPromise = getServiceDataSource("model") === "api"
      ? fetchWorkspaceSelectOptions(activeWorkspaceCode)
      : Promise.resolve(getWorkspaceSelectOptions(activeWorkspaceCode));

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
        toast.error(error instanceof Error ? error.message : "Execution options could not be loaded");
      })
      .finally(() => {
        if (active) setExecutionOptionsLoading(false);
      });

    return () => {
      active = false;
    };
  }, [activeWorkspaceCode, toast]);

  useEffect(() => {
    const allowed = new Set(effectiveUsers.map((user) => user.userCode));
    setAssigneeUserCodes((current) => current.filter((userCode) => allowed.has(userCode)));
  }, [effectiveUsers]);

  useEffect(() => {
    const allowed = new Set(appOptions.map((app) => app.appCode));
    setSelectedAppCodes((current) => current.filter((appCode) => allowed.has(appCode)));
  }, [appOptions]);

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

  async function submitTask() {
    if (submitting) return;

    if (!activeWorkspaceCode) {
      toast.error("Select a workspace in the shell header before creating a task");
      return;
    }

    setSubmitting(true);
    const payload = {
      agentCode: selectedAgentCode,
      agentCodes: splitCodes(agentCodesInput),
      appCodes: selectedAppCodes,
      assigneeUserCodes,
      description,
      modelCode: selectedModelCode,
      priority,
      sourceType,
      sourceUrl,
      taskType,
      title,
      workspaceCode: activeWorkspaceCode,
    };

    try {
      const created = dataSource === "api" ? await createTask(payload) : createMockTask(payload);
      toast.success(`Task ${created.taskCode} created`);
      navigate(`/tasks/detail?taskCode=${encodeURIComponent(created.taskCode)}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Task could not be created";
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "New" }]}
        title="Create Task"
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/tasks")}>Cancel</Button>
            <Button disabled={submitting || !title.trim() || !activeWorkspaceCode} aria-busy={submitting} onClick={submitTask}>
              <Save size={15} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        )}
      />
      <div className="space-y-6">
        <Field label="Task Title">
          <TextInput aria-label="Task title" className="w-full" value={title} onChange={(event) => setTitle(event.target.value)} />
        </Field>
        <div className="grid gap-6 xl:grid-cols-2">
          <Field label="Priority">
            <SelectInput aria-label="Priority" value={priority} onChange={(event) => setPriority(event.target.value as TaskPriority)}>
              {options.priorities.map((item) => <option key={item}>{item}</option>)}
            </SelectInput>
          </Field>
        </div>
        <div className="grid gap-4 lg:grid-cols-2">
          <Field label="Task Type">
            <SelectInput aria-label="Task Type" value={taskType} onChange={(event) => setTaskType(event.target.value as TaskType)}>
              {options.taskTypes.map((item) => <option key={item}>{item}</option>)}
            </SelectInput>
          </Field>
          <Field label="Source Type">
            <SelectInput aria-label="Source Type" value={sourceType} onChange={(event) => setSourceType(event.target.value as TaskSourceType)}>
              {options.sourceTypes.map((item) => <option key={item}>{item}</option>)}
            </SelectInput>
          </Field>
          <Field label="Source URL">
            <TextInput aria-label="Source URL" placeholder="Optional GitHub/GitLab URL" value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
          </Field>
          <Field label="Execution Agent">
            <OptionSelect
              ariaLabel="Task execution agent"
              disabled={executionOptionsLoading || !activeWorkspaceCode}
              loading={executionOptionsLoading}
              loadingLabel="Loading agents..."
              options={executionAgentSelectOptions}
              value={selectedAgentCode}
              onChange={setSelectedAgentCode}
            />
          </Field>
          <Field label="Execution Model">
            <OptionSelect
              ariaLabel="Task execution model"
              emptyLabel="No executable models"
              disabled={executionOptionsLoading || !activeWorkspaceCode || !modelOptions.length}
              loading={executionOptionsLoading}
              loadingLabel="Loading models..."
              options={executionModelSelectOptions}
              value={selectedModelOptionCode}
              onChange={setSelectedModelOptionCode}
            />
          </Field>
          <Field label="Agent Codes">
            <TextInput aria-label="Agent Codes" placeholder="agent-reviewer, agent-builder" value={agentCodesInput} onChange={(event) => setAgentCodesInput(event.target.value)} />
          </Field>
          <div className="lg:col-span-2">
            <Field label="Apps" hint="Select active apps from the current workspace.">
              <TaskAppSelectionField
                availableApps={appOptions}
                emptyMessage={activeWorkspaceCode ? "No active apps are available in this workspace." : "Select a workspace in the shell header to choose apps."}
                loading={workspaceOptionsLoading}
                selectedAppCodes={selectedAppCodes}
                onToggle={(appCode) => setSelectedAppCodes((current) => toggleCode(current, appCode))}
              />
            </Field>
          </div>
        </div>
        <Field label="Assigned Users" hint="Only effective users from the current workspace can be assigned.">
          <MultiSelectDropdown
            ariaLabel="Task assignees"
            emptyMessage={activeWorkspaceCode ? "No effective users available for the active workspace." : "Select a workspace in the shell header to assign users."}
            loading={workspaceOptionsLoading}
            loadingMessage="Loading user options..."
            options={userOptions}
            placeholder="Select assigned users"
            selectedValues={assigneeUserCodes}
            onToggle={(userCode) => setAssigneeUserCodes((current) => toggleCode(current, userCode))}
          />
        </Field>
        <SurfaceCard>
          <div className="grid gap-1.5">
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
            <TextArea
              aria-label="Task description"
              className="min-h-36 w-full"
              placeholder="Describe the outcome, expected deliverables, context, constraints, and variable references like $VAR1..."
              value={description}
              onChange={(event) => setDescription(event.target.value)}
            />
          </div>
        </SurfaceCard>
      </div>
      {descriptionFullscreenOpen ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label="Description fullscreen editor"
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Description</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  Write the objective, deliverables, context, constraints, and variable references in a larger workspace.
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
              <TextArea
                aria-label="Task description fullscreen"
                className="h-full min-h-0 w-full resize-none"
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
