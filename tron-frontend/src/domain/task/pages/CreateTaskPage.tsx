import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  Maximize2,
  Save,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Field, SelectInput, TextArea, TextInput } from "@/components/forms";
import { OptionSelect } from "@/components/option-select";
import {
  PageHeader,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import { createTask } from "@/domain/task/api";
import { createMockTask } from "@/domain/task/mock/data";
import {
  useTaskOptions,
  useTaskWorkspaceOptions,
} from "@/domain/task/hooks";
import { TaskAppSelectionField, TaskExecutionModelSelect } from "@/domain/task/components";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { fetchWorkspaceInternalAgents, fetchWorkspaceSystemAgents } from "@/domain/agent/api";
import { listWorkspaceInternalAgentsMock, listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import { fetchWorkspaceSelectOptions } from "@/domain/model/api/modelApi";
import { getWorkspaceSelectOptions } from "@/domain/model/mock/data";
import {
  automationModelCodeFromOption,
  automationModelOptionsForAgent,
  availableAutomationAgents,
  mainAutomationAgentOption,
  optionCodeForAutomationModel,
} from "@/domain/automation/pages/automationExecutionOptions";
import type { AutomationAgentOption } from "@/domain/automation/pages/automationExecutionOptions";
import type {
  TaskPriority,
  TaskSourceType,
  TaskType,
} from "@/domain/task/model";
import { SelectItem } from "@/components/ui/select";

function toggleCode(codes: string[], code: string) {
  return codes.includes(code) ? codes.filter((item) => item !== code) : [...codes, code];
}

export function CreateTaskPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`tasks.${key}`, { defaultValue, ...options })
  );
  const automaticModelLabel = pageT("values.automaticModel", "Automatic");
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [taskType, setTaskType] = useState<TaskType>("Feature");
  const [sourceType, setSourceType] = useState<TaskSourceType>("Internal");
  const [sourceUrl, setSourceUrl] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("Medium");
  const [agentOptions, setAgentOptions] = useState<AutomationAgentOption[]>(() => [mainAutomationAgentOption]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(mainAutomationAgentOption.agentCode);
  const [selectedModelOptionCode, setSelectedModelOptionCode] = useState(() =>
    optionCodeForAutomationModel(mainAutomationAgentOption.agentCode, null, "workspace"),
  );
  const [workspaceModelOptions, setWorkspaceModelOptions] = useState<ReturnType<typeof getWorkspaceSelectOptions>>(() => []);
  const [executionOptionsLoading, setExecutionOptionsLoading] = useState(false);
  const [selectedAppCodes, setSelectedAppCodes] = useState<string[]>([]);
  const [descriptionFullscreenOpen, setDescriptionFullscreenOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("task");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const options = useTaskOptions(activeWorkspaceCode);
  const { appOptions, loading: workspaceOptionsLoading } = useTaskWorkspaceOptions(activeWorkspaceCode);
  const executionAgentSelectOptions = useMemo(
    () => agentOptions.map((agent) => ({
      label: `${agent.name} (${agent.agentCode})`,
      value: agent.agentCode,
    })),
    [agentOptions],
  );
  const selectedAgent = useMemo(
    () => agentOptions.find((agent) => agent.agentCode === selectedAgentCode) ?? mainAutomationAgentOption,
    [agentOptions, selectedAgentCode],
  );
  const modelOptions = useMemo(
    () => automationModelOptionsForAgent(workspaceModelOptions, selectedAgent, automaticModelLabel),
    [automaticModelLabel, selectedAgent, workspaceModelOptions],
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
    const internalAgentsPromise = getServiceDataSource("agent") === "api"
      ? fetchWorkspaceInternalAgents(activeWorkspaceCode)
      : Promise.resolve(listWorkspaceInternalAgentsMock(activeWorkspaceCode));
    const modelsPromise = getServiceDataSource("model") === "api"
      ? fetchWorkspaceSelectOptions(activeWorkspaceCode)
      : Promise.resolve(getWorkspaceSelectOptions(activeWorkspaceCode));

    Promise.all([agentsPromise, internalAgentsPromise, modelsPromise])
      .then(([agents, internalAgents, models]) => {
        if (!active) return;
        setAgentOptions(availableAutomationAgents(agents, internalAgents));
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
  }, [activeWorkspaceCode, toast]);

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
      toast.error(pageT("errors.workspaceRequired", "Select a workspace in the shell header before creating a task"));
      return;
    }

    setSubmitting(true);
    const payload = {
      agentCode: selectedAgentCode,
      agentCodes: [],
      appCodes: selectedAppCodes,
      assigneeUserCodes: [],
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
      toast.success(pageT("messages.created", "Task {{code}} created", { code: created.taskCode }));
      navigate(`/tasks/detail?taskCode=${encodeURIComponent(created.taskCode)}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : pageT("errors.create", "Task could not be created");
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pageT("title", "Tasks"), href: "/tasks" }, { label: pageT("actions.new", "New") }]}
        title={pageT("detail.create", "Create Task")}
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => navigate("/tasks")}>{pageT("actions.cancel", "Cancel")}</Button>
            <Button size="lg" disabled={submitting || !title.trim() || !activeWorkspaceCode} aria-busy={submitting} onClick={submitTask}>
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </>
        )}
      />
      <div className="space-y-6">
        <Field label={pageT("fields.title", "Task Title")} required>
          <TextInput aria-label={pageT("fields.title", "Task title")} aria-required="true" className="w-full" required value={title} onChange={(event) => setTitle(event.target.value)} />
        </Field>
        <div className="grid gap-6 xl:grid-cols-2">
          <Field label={pageT("fields.priority", "Priority")} required>
            <SelectInput aria-label={pageT("fields.priority", "Priority")} aria-required="true" required value={priority} onValueChange={(event) => setPriority(event as TaskPriority)}>
              {options.priorities.map((item) => <SelectItem key={item} value={item}>{pageT(`priorities.${item}`, item)}</SelectItem>)}
            </SelectInput>
          </Field>
        </div>
        <div className="grid gap-4 lg:grid-cols-2">
          <Field label={pageT("fields.taskType", "Task Type")} required>
            <SelectInput aria-label={pageT("fields.taskType", "Task Type")} aria-required="true" required value={taskType} onValueChange={(event) => setTaskType(event as TaskType)}>
              {options.taskTypes.map((item) => <SelectItem key={item} value={item}>{pageT(`types.${item}`, item)}</SelectItem>)}
            </SelectInput>
          </Field>
          <Field label={pageT("fields.sourceType", "Source Type")} required>
            <SelectInput aria-label={pageT("fields.sourceType", "Source Type")} aria-required="true" required value={sourceType} onValueChange={(event) => setSourceType(event as TaskSourceType)}>
              {options.sourceTypes.map((item) => <SelectItem key={item} value={item}>{pageT(`sourceTypes.${item}`, item)}</SelectItem>)}
            </SelectInput>
          </Field>
          <Field label={pageT("fields.sourceUrl", "Source URL")}>
            <TextInput aria-label={pageT("fields.sourceUrl", "Source URL")} placeholder={pageT("placeholders.sourceUrl", "Optional GitHub/GitLab URL")} value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} />
          </Field>
          <Field label={pageT("fields.executionAgent", "Execution Agent")}>
            <OptionSelect
              ariaLabel={pageT("aria.executionAgent", "Task execution agent")}
              disabled={executionOptionsLoading || !activeWorkspaceCode}
              loading={executionOptionsLoading}
              loadingLabel={pageT("values.loadingAgents", "Loading agents...")}
              options={executionAgentSelectOptions}
              value={selectedAgentCode}
              onChange={setSelectedAgentCode}
            />
          </Field>
          <Field label={pageT("fields.executionModel", "Execution Model")}>
            <TaskExecutionModelSelect
              ariaLabel={pageT("aria.executionModel", "Task execution model")}
              emptyLabel={pageT("values.noModels", "No executable models")}
              disabled={executionOptionsLoading || !activeWorkspaceCode || !modelOptions.length}
              loading={executionOptionsLoading}
              loadingLabel={pageT("values.loadingModels", "Loading models...")}
              options={modelOptions}
              value={selectedModelOptionCode}
              onChange={setSelectedModelOptionCode}
            />
          </Field>
          <Field label={pageT("fields.apps", "Apps")} hint={pageT("hints.apps", "Select active apps from the current workspace.")}>
            <TaskAppSelectionField
              availableApps={appOptions}
              emptyMessage={activeWorkspaceCode ? pageT("values.noActiveApps", "No active apps are available in this workspace.") : pageT("errors.workspaceRequiredApps", "Select a workspace in the shell header to choose apps.")}
              loading={workspaceOptionsLoading}
              selectedAppCodes={selectedAppCodes}
              onToggle={(appCode) => setSelectedAppCodes((current) => toggleCode(current, appCode))}
            />
          </Field>
        </div>
        <SurfaceCard className="rounded-[4px]">
          <div className="grid gap-1.5">
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
            <TextArea
              aria-label={pageT("aria.description", "Task description")}
              className="min-h-36 w-full"
              placeholder={pageT("placeholders.description", "Describe the outcome, expected deliverables, context, constraints, and variable references like $VAR1...")}
              value={description}
              onChange={(event) => setDescription(event.target.value)}
            />
          </div>
        </SurfaceCard>
      </div>
      {descriptionFullscreenOpen ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-[var(--tron-overlay-bg,var(--color-backdrop-scrim))] p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog" data-theme-anchor="dialog"
            aria-modal="true"
            aria-label={pageT("aria.descriptionFullscreen", "Description fullscreen editor")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("fields.description", "Description")}</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {pageT("descriptionFullscreenHint", "Write the objective, deliverables, context, constraints, and variable references in a larger workspace.")}
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
              <TextArea
                aria-label={pageT("aria.descriptionFullscreen", "Task description fullscreen")}
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
