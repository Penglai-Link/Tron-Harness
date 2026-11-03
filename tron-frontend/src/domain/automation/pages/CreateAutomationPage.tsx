import { Maximize2, X } from "lucide-react";
import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import { Field, SelectInput, TextArea, TextInput } from "@/components/forms";
import { OptionSelect } from "@/components/option-select";
import { PageHeader } from "@/shared/layout/PageHeader";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { useToast } from "@/components/ui/toast";
import { createAutomation } from "@/domain/automation/api";
import { editableAutomationStatuses } from "@/domain/automation/constants";
import { createMockAutomation } from "@/domain/automation/mock/data";
import type { AutomationOperationalStatus } from "@/domain/automation/model";
import { fetchWorkspaceInternalAgents, fetchWorkspaceSystemAgents } from "@/domain/agent/api/agentManagementApi";
import { listWorkspaceInternalAgentsMock, listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import { fetchModelSelectOptions } from "@/domain/model/api/modelApi";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  automationModelCodeFromOption,
  automationModelOptionsForAgent,
  availableAutomationAgents,
  mainAutomationAgentOption,
  type AutomationAgentOption,
} from "./automationExecutionOptions";
import { SelectItem } from "@/components/ui/select";

export function CreateAutomationPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`automationCreate.${key}`, { defaultValue, ...options })
  );
  const dataSource = getServiceDataSource("automation");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [cronExpression, setCronExpression] = useState("0 9 * * *");
  const [status, setStatus] = useState<AutomationOperationalStatus>("Active");
  const [prompt, setPrompt] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [promptFullscreenOpen, setPromptFullscreenOpen] = useState(false);
  const [agentOptions, setAgentOptions] = useState<AutomationAgentOption[]>([mainAutomationAgentOption]);
  const [workspaceModelOptions, setWorkspaceModelOptions] = useState<Awaited<ReturnType<typeof fetchModelSelectOptions>>>([]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(mainAutomationAgentOption.agentCode);
  const [selectedModelOptionCode, setSelectedModelOptionCode] = useState("");
  const [executionOptionsLoading, setExecutionOptionsLoading] = useState(false);

  const selectedAgent = useMemo(
    () => agentOptions.find((agent) => agent.agentCode === selectedAgentCode) ?? mainAutomationAgentOption,
    [agentOptions, selectedAgentCode],
  );
  const executionAgentSelectOptions = useMemo(
    () => agentOptions.map((agent) => ({
      label: `${agent.name} (${agent.agentCode})`,
      value: agent.agentCode,
    })),
    [agentOptions],
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
      if (selectedModelOptionCode) setSelectedModelOptionCode("");
      return;
    }
    if (modelOptions.some((option) => option.optionCode === selectedModelOptionCode)) return;
    setSelectedModelOptionCode(modelOptions[0].optionCode);
  }, [modelOptions, selectedModelOptionCode]);

  useEffect(() => {
    if (!activeWorkspaceCode) {
      setAgentOptions([]);
      setWorkspaceModelOptions([]);
      setSelectedAgentCode(mainAutomationAgentOption.agentCode);
      setSelectedModelOptionCode("");
      return;
    }

    const controller = new AbortController();
    setExecutionOptionsLoading(true);

    const agentsPromise = dataSource === "api"
      ? fetchWorkspaceSystemAgents(activeWorkspaceCode, controller.signal)
      : Promise.resolve(listWorkspaceSystemAgentsMock(activeWorkspaceCode));
    const internalAgentsPromise = dataSource === "api"
      ? fetchWorkspaceInternalAgents(activeWorkspaceCode, controller.signal)
      : Promise.resolve(listWorkspaceInternalAgentsMock(activeWorkspaceCode));
    const modelsPromise = fetchModelSelectOptions(activeWorkspaceCode, controller.signal);

    Promise.all([agentsPromise, modelsPromise, internalAgentsPromise])
      .then(([systemAgents, workspaceModels, internalAgents]) => {
        if (controller.signal.aborted) return;
        setAgentOptions(availableAutomationAgents(systemAgents, internalAgents));
        setWorkspaceModelOptions(workspaceModels);
      })
      .catch((error) => {
        if (controller.signal.aborted) return;
        setAgentOptions([]);
        setWorkspaceModelOptions([]);
        toast.error(error instanceof Error ? error.message : pageT("errors.executionOptions", "Execution options could not be loaded"));
      })
      .finally(() => {
        if (!controller.signal.aborted) setExecutionOptionsLoading(false);
      });

    return () => controller.abort();
  }, [activeWorkspaceCode, dataSource, toast]);

  useEffect(() => {
    if (!promptFullscreenOpen) return;

    const originalOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setPromptFullscreenOpen(false);
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [promptFullscreenOpen]);

  async function submitAutomation() {
    if (submitting) return;
    if (!activeWorkspaceCode) {
      toast.error(pageT("errors.workspaceRequired", "Select a workspace in the shell header before creating an automation"));
      return;
    }

    setSubmitting(true);

    try {
      const created = dataSource === "api"
        ? await createAutomation({
          cronExpression,
          description,
          agentCode: selectedAgentCode,
          modelCode: selectedModelCode,
          name,
          prompt,
          status,
          workspaceCode: activeWorkspaceCode,
        })
        : createMockAutomation({
          cronExpression,
          description,
          agentCode: selectedAgentCode,
          modelCode: selectedModelCode,
          name,
          prompt,
          status,
          workspaceCode: activeWorkspaceCode,
        });
      toast.success(pageT("messages.created", "Automation {{code}} created", { code: created.automationCode }));
      navigate(`/automations/detail?automationCode=${encodeURIComponent(created.automationCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.create", "Automation could not be created"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pageT("title", "Automations"), href: "/automations" }, { label: pageT("actions.new", "New") }]}
        title={pageT("detail.create", "Create Automation")}
        actions={
          <>
            <Button size="lg" variant="outline" onClick={() => navigate("/automations")}>{pageT("actions.cancel", "Cancel")}</Button>
            <Button size="lg" disabled={submitting || !activeWorkspaceCode || !name.trim() || !prompt.trim() || !selectedModelOptionCode} aria-busy={submitting} onClick={submitAutomation}>
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </>
        }
      />
      <div>
        <SurfaceCard className="space-y-5">
          <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_240px]">
            <Field label={pageT("fields.name", "Automation name")}>
              <TextInput aria-label={pageT("fields.name", "Automation name")} placeholder={pageT("placeholders.name", "Daily workspace release digest")} value={name} onChange={(event) => setName(event.target.value)} />
            </Field>
            <Field label={pageT("fields.status", "Automation status")}>
              <SelectInput aria-label={pageT("fields.status", "Automation status")} value={status} onValueChange={(event) => setStatus(event as AutomationOperationalStatus)}>
                {editableAutomationStatuses.map((item) => (
                  <SelectItem key={item} value={item}>{pageT(`status.${item}`, item)}</SelectItem>
                ))}
              </SelectInput>
            </Field>
          </div>
          <Field label={pageT("fields.description", "Description")}>
            <TextArea aria-label={pageT("aria.description", "Automation description")} className="min-h-24" placeholder={pageT("placeholders.description", "Summarize the business intent, constraints, or expected operator-facing output.")} value={description} onChange={(event) => setDescription(event.target.value)} />
          </Field>
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={pageT("fields.executionAgent", "Execution agent")}>
              <OptionSelect
                ariaLabel={pageT("aria.executionAgent", "Automation execution agent")}
                disabled={executionOptionsLoading || !activeWorkspaceCode}
                loading={executionOptionsLoading}
                loadingLabel={pageT("values.loadingAgents", "Loading agents...")}
                options={executionAgentSelectOptions}
                value={selectedAgentCode}
                onChange={(value) => {
                  setSelectedAgentCode(value);
                  setSelectedModelOptionCode("");
                }}
              />
            </Field>
            <Field label={pageT("fields.executionModel", "Execution model")}>
              <OptionSelect
                ariaLabel={pageT("aria.executionModel", "Automation execution model")}
                emptyLabel={pageT("values.noModels", "No executable models")}
                disabled={executionOptionsLoading || !activeWorkspaceCode || !modelOptions.length}
                loading={executionOptionsLoading}
                loadingLabel={pageT("values.loadingModels", "Loading models...")}
                options={executionModelSelectOptions}
                value={selectedModelOptionCode}
                onChange={setSelectedModelOptionCode}
              />
            </Field>
          </div>
          <Field label={pageT("fields.cron", "Cron expression")}>
            <TextInput aria-label={pageT("fields.cron", "Cron expression")} className="font-mono" placeholder="*/15 * * * *" value={cronExpression} onChange={(event) => setCronExpression(event.target.value)} />
          </Field>
          <div className="grid gap-1.5">
            <div className="flex items-center justify-between gap-3">
              <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                {pageT("fields.prompt", "Prompt body")}
              </span>
              <Button aria-label={pageT("aria.openPrompt", "Fullscreen")} size="lg" type="button" variant="ghost" onClick={() => setPromptFullscreenOpen(true)}>
                <Maximize2 size={14} aria-hidden="true" />
              </Button>
            </div>
            <TextArea
              aria-label={pageT("aria.prompt", "Automation prompt")}
              className="min-h-80 w-full"
              placeholder={pageT("placeholders.prompt", "Write the long-form prompt that the automation should dispatch for this workspace.")}
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
            />
          </div>
        </SurfaceCard>
      </div>
      {promptFullscreenOpen ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-[var(--tron-overlay-bg,var(--color-backdrop-scrim))] p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog" data-theme-anchor="dialog"
            aria-modal="true"
            aria-label={pageT("aria.promptFullscreen", "Prompt fullscreen editor")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("fields.automationPrompt", "Automation prompt")}</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {pageT("promptFullscreenHint", "Write the long-form prompt that will be dispatched whenever this automation runs.")}
                </p>
              </div>
              <Button size="icon" type="button" variant="ghost" aria-label={pageT("aria.closePrompt", "Close prompt fullscreen")} onClick={() => setPromptFullscreenOpen(false)}>
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              <TextArea
                aria-label={pageT("aria.promptFullscreen", "Automation prompt fullscreen")}
                className="h-full min-h-0 w-full resize-none"
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
              />
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
