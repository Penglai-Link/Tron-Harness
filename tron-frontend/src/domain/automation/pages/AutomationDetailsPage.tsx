import { X } from "lucide-react";
import { getUserTimeZone } from "@/shared/lib/userTimeZone";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  useNavigate,
  useSearchParams,
} from "react-router";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DetailField } from "@/components/detail-field";
import { Field, TextArea, TextInput } from "@/components/forms";
import { MarkdownContent } from "@/components/markdown-content";
import { OptionSelect } from "@/components/option-select";
import {
  PageHeader,
  SurfaceCard,
  useLocaleText,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  archiveAutomation,
  executeAutomation,
  restoreAutomation,
  updateAutomation,
  updateAutomationStatus,
} from "@/domain/automation/api";
import { getAutomationStatusTone } from "@/domain/automation/constants";
import {
  useAutomationDetail,
} from "@/domain/automation/hooks";
import {
  archiveMockAutomation,
  executeMockAutomation,
  restoreMockAutomation,
  updateMockAutomation,
  updateMockAutomationStatus,
} from "@/domain/automation/mock/data";
import type {
  AutomationOperationalStatus,
} from "@/domain/automation/model";
import { fetchWorkspaceSystemAgents } from "@/domain/agent/api/agentManagementApi";
import { listWorkspaceSystemAgentsMock } from "@/domain/agent/mock";
import { fetchWorkspaceSelectOptions } from "@/domain/model/api/modelApi";
import { getWorkspaceSelectOptions } from "@/domain/model/mock/data";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  automationModelCodeFromOption,
  automationModelOptionsForAgent,
  availableAutomationAgents,
  mainAutomationAgentOption,
  optionCodeForAutomationModel,
  type AutomationAgentOption,
  type AutomationModelOption,
} from "./automationExecutionOptions";

function formatTimestamp(value?: string | null) {
  if (!value) return "Not available";
  return new Intl.DateTimeFormat("en-US", { timeZone: getUserTimeZone(),
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function ReadValue({ children }: { children: ReactNode }) {
  const pt = useLocaleText();
  return (
    <div className="min-h-6 py-1 text-sm leading-6 text-[var(--color-on-surface)] whitespace-pre-wrap">
      {children || <span className="text-[var(--color-on-surface-variant)]">{pt("None")}</span>}
    </div>
  );
}

function automationExecutionsHref(automationCode: string) {
  const params = new URLSearchParams({ automationCode });
  return `/automations/executions?${params.toString()}`;
}

export function AutomationDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const pt = useLocaleText();
  const automationCode = searchParams.get("automationCode") ?? "";
  const { automation, refresh, setAutomation } = useAutomationDetail(automationCode);
  const toast = useToast();
  const dataSource = getServiceDataSource("automation");
  const [name, setName] = useState(automation.name);
  const [description, setDescription] = useState(automation.description ?? "");
  const [cronExpression, setCronExpression] = useState(automation.cronExpression);
  const [prompt, setPrompt] = useState(automation.prompt);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [statusBusy, setStatusBusy] = useState(false);
  const [executionBusy, setExecutionBusy] = useState(false);
  const [archiveBusy, setArchiveBusy] = useState(false);
  const [promptFullscreenOpen, setPromptFullscreenOpen] = useState(false);
  const [agentOptions, setAgentOptions] = useState<AutomationAgentOption[]>([mainAutomationAgentOption]);
  const [workspaceModelOptions, setWorkspaceModelOptions] = useState<Awaited<ReturnType<typeof fetchWorkspaceSelectOptions>>>([]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(automation.agentCode || mainAutomationAgentOption.agentCode);
  const [selectedModelOptionCode, setSelectedModelOptionCode] = useState(optionCodeForAutomationModel(automation.agentCode, automation.modelCode));
  const [executionOptionsLoading, setExecutionOptionsLoading] = useState(false);
  const automationRef = useRef(automation);
  automationRef.current = automation;
  const archived = automation.status === "Archived" || Boolean(automation.archivedAt);
  const selectedAgent = useMemo(() => {
    const matchedAgent = agentOptions.find((agent) => agent.agentCode === selectedAgentCode);
    if (matchedAgent) return matchedAgent;
    if (!selectedAgentCode || selectedAgentCode === mainAutomationAgentOption.agentCode) {
      return mainAutomationAgentOption;
    }
    return {
      agentCode: selectedAgentCode,
      description: "Configured automation agent.",
      models: [],
      name: selectedAgentCode,
    };
  }, [agentOptions, selectedAgentCode]);
  const visibleAgentOptions = useMemo(() => {
    if (agentOptions.some((agent) => agent.agentCode === selectedAgent.agentCode)) {
      return agentOptions;
    }
    return [selectedAgent, ...agentOptions];
  }, [agentOptions, selectedAgent]);
  const executionAgentSelectOptions = useMemo(
    () => visibleAgentOptions.map((agent) => ({
      label: `${agent.name} (${agent.agentCode})`,
      value: agent.agentCode,
    })),
    [visibleAgentOptions],
  );
  const modelOptions = useMemo(
    () => automationModelOptionsForAgent(workspaceModelOptions, selectedAgent),
    [selectedAgent, workspaceModelOptions],
  );
  const visibleModelOptions = useMemo<AutomationModelOption[]>(() => {
    if (!selectedModelOptionCode || modelOptions.some((option) => option.optionCode === selectedModelOptionCode)) {
      return modelOptions;
    }
    if (!automation.modelCode) return modelOptions;
    return [
      {
        label: automation.modelCode,
        modelCode: automation.modelCode,
        optionCode: selectedModelOptionCode,
      },
      ...modelOptions,
    ];
  }, [automation.modelCode, modelOptions, selectedModelOptionCode]);
  const executionModelSelectOptions = useMemo(
    () => visibleModelOptions.map((model) => ({
      label: model.label,
      value: model.optionCode,
    })),
    [visibleModelOptions],
  );
  const selectedModelCode = useMemo(
    () => automationModelCodeFromOption(visibleModelOptions, selectedModelOptionCode),
    [selectedModelOptionCode, visibleModelOptions],
  );

  useEffect(() => {
    setName(automation.name);
    setDescription(automation.description ?? "");
    setCronExpression(automation.cronExpression);
    setPrompt(automation.prompt);
    setSelectedAgentCode(automation.agentCode || mainAutomationAgentOption.agentCode);
    setSelectedModelOptionCode(optionCodeForAutomationModel(automation.agentCode, automation.modelCode));
    setEditing(false);
    setPromptFullscreenOpen(false);
  }, [automation]);

  useEffect(() => {
    if (visibleAgentOptions.some((agent) => agent.agentCode === selectedAgentCode)) return;
    setSelectedAgentCode(mainAutomationAgentOption.agentCode);
  }, [selectedAgentCode, visibleAgentOptions]);

  useEffect(() => {
    if (!visibleModelOptions.length) {
      if (selectedModelOptionCode && selectedModelCode !== automation.modelCode) {
        setSelectedModelOptionCode("");
      }
      return;
    }
    if (visibleModelOptions.some((option) => option.optionCode === selectedModelOptionCode)) return;
    setSelectedModelOptionCode(visibleModelOptions[0].optionCode);
  }, [automation.modelCode, selectedModelCode, selectedModelOptionCode, visibleModelOptions]);

  useEffect(() => {
    const workspaceCode = automation.workspaceCode.trim();
    if (!workspaceCode) {
      setAgentOptions([mainAutomationAgentOption]);
      setWorkspaceModelOptions([]);
      return;
    }

    const controller = new AbortController();
    setExecutionOptionsLoading(true);

    const agentsPromise = dataSource === "api"
      ? fetchWorkspaceSystemAgents(workspaceCode, controller.signal)
      : Promise.resolve(listWorkspaceSystemAgentsMock(workspaceCode));
    const modelsPromise = dataSource === "api"
      ? fetchWorkspaceSelectOptions(workspaceCode, controller.signal)
      : Promise.resolve(getWorkspaceSelectOptions(workspaceCode));

    Promise.all([agentsPromise, modelsPromise])
      .then(([systemAgents, workspaceModels]) => {
        if (controller.signal.aborted) return;
        setAgentOptions(availableAutomationAgents(systemAgents));
        setWorkspaceModelOptions(workspaceModels);
      })
      .catch((error) => {
        if (controller.signal.aborted) return;
        setAgentOptions([mainAutomationAgentOption]);
        setWorkspaceModelOptions([]);
        toast.error(error instanceof Error ? error.message : "Execution options could not be loaded");
      })
      .finally(() => {
        if (!controller.signal.aborted) setExecutionOptionsLoading(false);
      });

    return () => controller.abort();
  }, [automation.workspaceCode, dataSource, toast]);

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

  const detailDirty = name !== automation.name
    || description !== (automation.description ?? "")
    || cronExpression !== automation.cronExpression
    || prompt !== automation.prompt
    || selectedAgentCode !== (automation.agentCode || mainAutomationAgentOption.agentCode)
    || selectedModelCode !== automation.modelCode;

  function resetDraft() {
    setName(automation.name);
    setDescription(automation.description ?? "");
    setCronExpression(automation.cronExpression);
    setPrompt(automation.prompt);
    setSelectedAgentCode(automation.agentCode || mainAutomationAgentOption.agentCode);
    setSelectedModelOptionCode(optionCodeForAutomationModel(automation.agentCode, automation.modelCode));
    setPromptFullscreenOpen(false);
    setEditing(false);
  }

  async function saveDetail() {
    if (saving || archived || !detailDirty) return;
    if (!selectedModelOptionCode) {
      toast.error(pt("Select an execution model before saving"));
      return;
    }
    setSaving(true);

    try {
      const payload = {
        agentCode: selectedAgentCode,
        cronExpression,
        description,
        modelCode: selectedModelCode,
        name,
        prompt,
      };
      const updated = dataSource === "api"
        ? await updateAutomation(automation.automationCode, payload)
        : updateMockAutomation(automation.automationCode, payload);
      setAutomation(updated);
      setEditing(false);
      setPromptFullscreenOpen(false);
      toast.success(pt("Automation {{code}} saved", { code: updated.automationCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Automation could not be saved"));
    } finally {
      setSaving(false);
    }
  }

  async function handleStatusChange(status: AutomationOperationalStatus) {
    if (statusBusy || archived || status === automation.status) return;
    setStatusBusy(true);

    try {
      const updated = dataSource === "api"
        ? await updateAutomationStatus(automation.automationCode, { status })
        : updateMockAutomationStatus(automation.automationCode, { status });
      setAutomation(updated);
      toast.success(pt("Automation {{code}} {{status}}", { code: updated.automationCode, status: status === "Active" ? pt("activated") : pt("paused") }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Automation status could not be updated"));
    } finally {
      setStatusBusy(false);
    }
  }

  async function handleArchive() {
    if (archiveBusy || archived) return;
    setArchiveBusy(true);

    try {
      const lifecycle = dataSource === "api"
        ? await archiveAutomation(automation.automationCode)
        : archiveMockAutomation(automation.automationCode);
      setAutomation({
        ...automationRef.current,
        archivedAt: lifecycle.archivedAt,
        previousStatus: lifecycle.previousStatus,
        status: lifecycle.status,
      });
      toast.success(pt("Automation {{code}} archived", { code: lifecycle.automationCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Automation could not be archived"));
    } finally {
      setArchiveBusy(false);
    }
  }

  async function handleRestore() {
    if (archiveBusy || !archived) return;
    setArchiveBusy(true);

    try {
      const lifecycle = dataSource === "api"
        ? await restoreAutomation(automation.automationCode)
        : restoreMockAutomation(automation.automationCode);
      await refresh();
      setAutomation({
        ...automationRef.current,
        archivedAt: lifecycle.archivedAt,
        previousStatus: lifecycle.previousStatus,
        status: lifecycle.status,
      });
      toast.success(pt("Automation {{code}} restored to {{status}}", { code: lifecycle.automationCode, status: pt(lifecycle.status) }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Automation could not be restored"));
    } finally {
      setArchiveBusy(false);
    }
  }

  async function handleExecute() {
    if (executionBusy || archived) return;
    setExecutionBusy(true);

    try {
      const updated = dataSource === "api"
        ? await executeAutomation(automation.automationCode)
        : executeMockAutomation(automation.automationCode);
      setAutomation(updated);
      toast.success(pt("Manual execution recorded for {{code}}", { code: updated.automationCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Automation execution could not be recorded"));
    } finally {
      setExecutionBusy(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: pt("Automations"), href: "/automations" },
          { label: automation.automationCode || pt("Detail") },
        ]}
        title={automation.name || pt("Automation Detail")}
        actions={(
          <>
            <div className="inline-flex items-center rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-1">
              <Button
                size="sm"
                disabled={archived || statusBusy || automation.status === "Active"}
                variant={automation.status === "Active" && !archived ? "default" : "ghost"}
                onClick={() => {
                  void handleStatusChange("Active");
                }}
              >

                {pt("Activate")}
              </Button>
              <Button
                size="sm"
                disabled={archived || statusBusy || automation.status === "Paused"}
                variant={automation.status === "Paused" && !archived ? "default" : "ghost"}
                onClick={() => {
                  void handleStatusChange("Paused");
                }}
              >

                {pt("Pause")}
              </Button>
            </div>
            <Button variant="secondary" onClick={() => navigate(automationExecutionsHref(automation.automationCode || automationCode))}>

              {pt("Execution instances")}
            </Button>
            <Button disabled={executionBusy || archived} variant="secondary" onClick={handleExecute}>

              {executionBusy ? pt("Running...") : pt("Run Now")}
            </Button>
            {archived ? (
              <Button disabled={archiveBusy} variant="secondary" onClick={handleRestore}>

                {archiveBusy ? pt("Restoring...") : pt("Restore")}
              </Button>
            ) : (
              <Button disabled={archiveBusy} variant="secondary" onClick={handleArchive}>

                {archiveBusy ? pt("Archiving...") : pt("Archive")}
              </Button>
            )}
            {editing ? (
              <>
                <Button variant="secondary" onClick={resetDraft}>{pt("Cancel")}</Button>
                <Button disabled={!detailDirty || saving || archived || !selectedModelOptionCode} aria-busy={saving} onClick={() => void saveDetail()}>

                  {saving ? pt("Saving...") : pt("Save")}
                </Button>
              </>
            ) : (
              <Button disabled={archived} variant="secondary" onClick={() => setEditing(true)}>

                {pt("Edit")}
              </Button>
            )}
          </>
        )}
      />
      <div className="space-y-5">
        <SurfaceCard>
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">{pt("Core definition")}</h2>
            </div>
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            <Field label={pt("Automation name")}>
              <TextInput aria-label={pt("Automation name")} disabled={!editing || archived} value={name} onChange={(event) => setName(event.target.value)} />
            </Field>
            <Field label={pt("Workspace")}>
              <TextInput aria-label={pt("Workspace code")} disabled value={automation.workspaceCode} />
            </Field>
            <Field label={pt("Execution agent")}>
              <OptionSelect
                ariaLabel={pt("Automation execution agent")}
                disabled={!editing || archived || executionOptionsLoading}
                loading={executionOptionsLoading}
                loadingLabel={pt("Loading agents...")}
                options={executionAgentSelectOptions}
                value={selectedAgentCode}
                onChange={(value) => {
                  setSelectedAgentCode(value);
                  setSelectedModelOptionCode("");
                }}
              />
            </Field>
            <Field label={pt("Execution model")}>
              <OptionSelect
                ariaLabel={pt("Automation execution model")}
                emptyLabel={pt("No executable models")}
                disabled={!editing || archived || executionOptionsLoading || !visibleModelOptions.length}
                loading={executionOptionsLoading}
                loadingLabel={pt("Loading models...")}
                options={executionModelSelectOptions}
                value={selectedModelOptionCode}
                onChange={setSelectedModelOptionCode}
              />
            </Field>
            <Field label={pt("Cron expression")}>
              <TextInput aria-label={pt("Cron expression")} className="font-mono" disabled={!editing || archived} value={cronExpression} onChange={(event) => setCronExpression(event.target.value)} />
            </Field>
            <Field label={pt("Scheduler expression")}>
              <TextInput aria-label={pt("Scheduler expression")} className="font-mono" disabled value={automation.schedulerExpression} />
            </Field>
          </div>
          <div className="mt-4">
            <Field label={pt("Description")}>
              <TextArea
                aria-label={pt("Automation description")}
                className="min-h-24"
                disabled={!editing || archived}
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </Field>
          </div>
          <div className="mt-5 grid gap-1.5 border-t border-[var(--color-outline-variant)] pt-4">
            <div className="flex flex-wrap items-center justify-between gap-[12px]">
              <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pt("Prompt body")}</span>
              <Button size="sm" type="button" variant="ghost" onClick={() => setPromptFullscreenOpen(true)}>

                {pt("Fullscreen")}
              </Button>
            </div>
            {editing ? (
              <TextArea
                aria-label={pt("Automation prompt")}
                className="min-h-80 w-full"
                disabled={archived}
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
              />
            ) : (
              <MarkdownContent
                ariaLabel="Automation prompt preview"
                className="max-h-80 overflow-y-auto pr-2"
                content={automation.prompt}
              />
            )}
          </div>
        </SurfaceCard>
        <SurfaceCard>
          <div className="mb-4">
            <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">{pt("Metadata")}</h2>
          </div>
          <div className="grid gap-4 lg:grid-cols-3">
            <DetailField label={pt("Automation code")} value={<ReadValue>{automation.automationCode}</ReadValue>} />
            <DetailField label={pt("Record ID")} value={<ReadValue>{automation.id}</ReadValue>} />
            <DetailField
              label={pt("Status")}
              value={<Badge variant={getAutomationStatusTone(automation.status)}>{pt(automation.status)}</Badge>}
            />
            <DetailField label={pt("Workspace")} value={<ReadValue>{automation.workspaceCode}</ReadValue>} />
            <DetailField label={pt("Agent code")} value={<ReadValue>{automation.agentCode}</ReadValue>} />
            <DetailField label={pt("Model code")} value={<ReadValue>{automation.modelCode}</ReadValue>} />
            <DetailField label={pt("Timezone")} value={<ReadValue>{automation.timezone}</ReadValue>} />
            <DetailField label={pt("Created")} value={formatTimestamp(automation.createdAt)} />
            <DetailField label={pt("Updated")} value={formatTimestamp(automation.updatedAt)} />
            <DetailField label={pt("Archived at")} value={formatTimestamp(automation.archivedAt)} />
            <DetailField label={pt("Previous status")} value={<ReadValue>{automation.previousStatus}</ReadValue>} />
          </div>
        </SurfaceCard>
      </div>
      {promptFullscreenOpen ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-[var(--tron-overlay-bg,var(--color-backdrop-scrim))] p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog" data-theme-anchor="dialog"
            aria-modal="true"
            aria-label={editing ? pt("Prompt fullscreen editor") : pt("Prompt fullscreen preview")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pt("Automation prompt")}</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {editing
                    ? pt("Edit the long-form prompt in a larger workspace.")
                    : pt("Preview the rendered prompt that the scheduler will dispatch for this automation.")}
                </p>
              </div>
              <Button size="icon" type="button" variant="ghost" aria-label={pt("Close prompt fullscreen")} onClick={() => setPromptFullscreenOpen(false)}>
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              {editing ? (
                <TextArea
                  aria-label={pt("Automation prompt fullscreen")}
                  className="h-full min-h-0 w-full resize-none"
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                />
              ) : (
                <div className="h-full overflow-y-auto pr-1">
                  <MarkdownContent ariaLabel={pt("Automation prompt fullscreen preview")} className="max-w-none" content={automation.prompt} />
                </div>
              )}
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
