import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Archive,
  Edit3,
  ExternalLink,
  Maximize2,
  MoreHorizontal,
  Pause,
  Play,
  RefreshCcw,
  Save,
  StepForward,
  Trash2,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  ConfirmDialog,
  DataTable,
  DetailField,
  Field,
  InspectorPanel,
  MarkdownContent,
  OptionSelect,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  useToast,
  useConsoleShellConfig,
  useDismissibleLayer,
  useLocaleText,
} from "@/shared";

import {
  archiveAutomation,
  deleteAutomationExecution,
  executeAutomation,
  restoreAutomation,
  updateAutomation,
  updateAutomationStatus,
} from "@/domain/automation/api";
import {
  getAutomationExecutionStatusTone,
} from "@/domain/automation/constants";
import {
  useAutomationDetail,
  useAutomationExecutions,
} from "@/domain/automation/hooks";
import {
  archiveMockAutomation,
  deleteMockAutomationExecution,
  executeMockAutomation,
  restoreMockAutomation,
  updateMockAutomation,
  updateMockAutomationStatus,
} from "@/domain/automation/mock/data";
import type {
  AutomationExecutionListItem,
  AutomationDetailData,
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

interface AutomationSubtask {
  progress: number;
  status: string;
  taskCode: string;
  title: string;
}

interface ExecutionRowMenuState {
  executionCode: string;
  left: number;
  top: number;
  triggerElement: HTMLElement;
}

function formatTimestamp(value?: string | null) {
  if (!value) return "Not available";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function ReadValue({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-6 py-1 text-sm leading-6 text-[var(--color-on-surface)] whitespace-pre-wrap">
      {children || <span className="text-[var(--color-on-surface-variant)]">None</span>}
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

function resolveSubtasks(automation: AutomationDetailData & { subtasks?: unknown }): AutomationSubtask[] {
  if (!Array.isArray(automation.subtasks)) {
    return [];
  }

  return automation.subtasks
    .filter((entry) => {
      return Boolean(
        typeof entry === "object"
        && entry !== null
        && "taskCode" in entry
        && "title" in entry
      );
    })
    .map((entry) => {
      const candidate = entry as Record<string, unknown>;
      return {
        progress: Number(candidate.progress ?? 0),
        status: String(candidate.status ?? ""),
        taskCode: String(candidate.taskCode ?? ""),
        title: String(candidate.title ?? ""),
      };
    });
}

function statusLabel(status: string) {
  return status || "Unknown";
}

function executionPageBounds(page: number, pageSize: number, total: number, visible: number) {
  if (!total || !visible) {
    return { end: 0, start: 0 };
  }
  const start = (page - 1) * pageSize + 1;
  return {
    end: start + visible - 1,
    start,
  };
}

function chatSessionDetailHref(sessionCode: string) {
  const params = new URLSearchParams({
    sessionCode,
    sourceTab: "automation",
  });
  return `/chats?${params.toString()}`;
}

export function AutomationDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const pt = useLocaleText();
  const automationCode = searchParams.get("automationCode") ?? "";
  const { automation, refresh, setAutomation } = useAutomationDetail(automationCode);
  const [executionCurrent, setExecutionCurrent] = useState(1);
  const [executionSize, setExecutionSize] = useState(15);
  const {
    loading: executionsLoading,
    page: executionPage,
    refresh: refreshExecutions,
  } = useAutomationExecutions(automationCode, { current: executionCurrent, size: executionSize });
  const executionPageState = executionPage ?? {
    current: 1,
    records: [] as AutomationExecutionListItem[],
    size: 15,
    total: 0,
  };
  const executionPageRecords = Array.isArray(executionPageState.records) ? executionPageState.records : [];
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
  const [executionRowBusyKey, setExecutionRowBusyKey] = useState("");
  const [openExecutionRowMenu, setOpenExecutionRowMenu] = useState<ExecutionRowMenuState | null>(null);
  const [deleteTargetExecution, setDeleteTargetExecution] = useState<AutomationExecutionListItem | null>(null);
  const [promptFullscreenOpen, setPromptFullscreenOpen] = useState(false);
  const [agentOptions, setAgentOptions] = useState<AutomationAgentOption[]>([mainAutomationAgentOption]);
  const [workspaceModelOptions, setWorkspaceModelOptions] = useState<Awaited<ReturnType<typeof fetchWorkspaceSelectOptions>>>([]);
  const [selectedAgentCode, setSelectedAgentCode] = useState(automation.agentCode || mainAutomationAgentOption.agentCode);
  const [selectedModelOptionCode, setSelectedModelOptionCode] = useState(optionCodeForAutomationModel(automation.agentCode, automation.modelCode));
  const [executionOptionsLoading, setExecutionOptionsLoading] = useState(false);
  const automationRef = useRef(automation);
  const executionRowMenuRef = useRef<HTMLDivElement | null>(null);
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

  useEffect(() => {
    setExecutionCurrent(1);
  }, [automationCode]);

  useEffect(() => {
    setOpenExecutionRowMenu(null);
  }, [automationCode, executionCurrent, executionSize]);

  useDismissibleLayer({
    onDismiss: () => setOpenExecutionRowMenu(null),
    open: openExecutionRowMenu !== null,
    popupRef: executionRowMenuRef,
    triggerElement: openExecutionRowMenu?.triggerElement,
  });

  const detailDirty = name !== automation.name
    || description !== (automation.description ?? "")
    || cronExpression !== automation.cronExpression
    || prompt !== automation.prompt
    || selectedAgentCode !== (automation.agentCode || mainAutomationAgentOption.agentCode)
    || selectedModelCode !== automation.modelCode;
  const executionPageCount = Math.max(1, Math.ceil(executionPageState.total / executionPageState.size));
  const executionBounds = executionPageBounds(
    executionPageState.current,
    executionPageState.size,
    executionPageState.total,
    executionPageRecords.length,
  );
  const subtasks = useMemo(() => resolveSubtasks(automation), [automation]);
  const executionInitialLoading = executionsLoading && executionPageState.total === 0;
  const openExecutionMenuRow = openExecutionRowMenu
    ? executionPageRecords.find((item) => item.executionCode === openExecutionRowMenu.executionCode) ?? null
    : null;
  const rightPanel = useMemo(() => (
    <InspectorPanel title="Automation Details">
      <div className="space-y-5">
        <div className="space-y-4">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Latest execution rollup</p>
          <DetailField
            label="Last result"
            value={automation.lastExecutionStatus ? <Badge tone={getAutomationExecutionStatusTone(automation.lastExecutionStatus)}>{automation.lastExecutionStatus}</Badge> : "Never executed"}
          />
          <DetailField label="Summary" value={<ReadValue>{automation.lastExecutionSummary}</ReadValue>} />
          <DetailField label="Execution code" value={<ReadValue>{automation.lastExecutionCode}</ReadValue>} />
          <DetailField label="Last started" value={formatTimestamp(automation.lastStartedAt)} />
          <DetailField label="Last finished" value={formatTimestamp(automation.lastFinishedAt)} />
          <DetailField label="Next run" value={formatTimestamp(automation.nextRunAt)} />
        </div>
        <div className="space-y-4">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Metadata</p>
          <DetailField label="Automation code" value={<ReadValue>{automation.automationCode}</ReadValue>} />
          <DetailField label="Agent" value={<ReadValue>{automation.agentCode}</ReadValue>} />
          <DetailField label="Model" value={<ReadValue>{automation.modelCode}</ReadValue>} />
          <DetailField label="Timezone" value={<ReadValue>{automation.timezone}</ReadValue>} />
          <DetailField label="Created" value={formatTimestamp(automation.createdAt)} />
          <DetailField label="Updated" value={formatTimestamp(automation.updatedAt)} />
          <DetailField label="Archived at" value={formatTimestamp(automation.archivedAt)} />
          <DetailField label="Previous status" value={<ReadValue>{automation.previousStatus}</ReadValue>} />
        </div>
        {subtasks.length ? (
          <div className="space-y-2">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Subtasks</p>
            <div className="space-y-2">
              {subtasks.map((item) => (
                <div className="rounded-lg border border-[var(--color-outline-variant)] p-3" key={item.taskCode}>
                  <div className="text-xs font-mono text-[var(--color-on-surface-variant)]">{item.taskCode}</div>
                  <div className="mt-1 text-sm font-medium">{item.title}</div>
                  <div className="mt-2 flex items-center justify-between gap-3">
                    <span className="text-xs text-[var(--color-on-surface-variant)]">{statusLabel(item.status)}</span>
                    <span className="text-xs text-[var(--color-on-surface-variant)]">{item.progress}%</span>
                  </div>
                  <div className="mt-2"><ProgressBar value={item.progress} /></div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <p className="text-sm text-[var(--color-on-surface-variant)]">No active subtasks.</p>
        )}
      </div>
    </InspectorPanel>
  ), [automation, subtasks]);

  useConsoleShellConfig({ rightPanel });

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
      await Promise.all([refresh(), refreshExecutions()]);
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
      refreshExecutions();
      toast.success(pt("Manual execution recorded for {{code}}", { code: updated.automationCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Automation execution could not be recorded"));
    } finally {
      setExecutionBusy(false);
    }
  }

  function handleRefreshExecutions() {
    refreshExecutions();
    toast.success(pt("Execution history refreshed"));
  }

  function toggleExecutionRowMenu(row: AutomationExecutionListItem, trigger: HTMLButtonElement) {
    if (executionRowBusyKey === `delete:${row.executionCode}`) return;
    if (openExecutionRowMenu?.executionCode === row.executionCode) {
      setOpenExecutionRowMenu(null);
      return;
    }

    const menuWidth = 184;
    const menuHeight = 112;
    const rect = trigger.getBoundingClientRect();
    const left = Math.min(
      Math.max(12, rect.right - menuWidth),
      window.innerWidth - menuWidth - 12,
    );
    const top = Math.min(
      Math.max(12, rect.bottom + 8),
      window.innerHeight - menuHeight - 12,
    );

    setOpenExecutionRowMenu({
      executionCode: row.executionCode,
      left,
      top,
      triggerElement: trigger,
    });
  }

  function handleOpenExecutionSession(row: AutomationExecutionListItem) {
    if (!row.sessionCode) return;
    setOpenExecutionRowMenu(null);
    navigate(chatSessionDetailHref(row.sessionCode));
  }

  function requestDeleteExecution(row: AutomationExecutionListItem) {
    setOpenExecutionRowMenu(null);
    setDeleteTargetExecution(row);
  }

  async function confirmDeleteExecution() {
    const target = deleteTargetExecution;
    if (!target || executionRowBusyKey) return;
    const busyKey = `delete:${target.executionCode}`;
    setExecutionRowBusyKey(busyKey);

    try {
      const deleted = dataSource === "api"
        ? await deleteAutomationExecution(target.automationCode, target.executionCode)
        : deleteMockAutomationExecution(target.automationCode, target.executionCode);
      setDeleteTargetExecution(null);
      await refresh();
      if (executionPageRecords.length === 1 && executionCurrent > 1) {
        setExecutionCurrent((current) => Math.max(1, current - 1));
      } else {
        refreshExecutions();
      }
      toast.success(pt("Execution {{code}} deleted", { code: deleted.executionCode }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pt("Execution could not be deleted"));
    } finally {
      setExecutionRowBusyKey("");
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
                variant={automation.status === "Active" && !archived ? "primary" : "ghost"}
                onClick={() => {
                  void handleStatusChange("Active");
                }}
              >
                <Play size={14} aria-hidden="true" />
                {pt("Activate")}
              </Button>
              <Button
                size="sm"
                disabled={archived || statusBusy || automation.status === "Paused"}
                variant={automation.status === "Paused" && !archived ? "primary" : "ghost"}
                onClick={() => {
                  void handleStatusChange("Paused");
                }}
              >
                <Pause size={14} aria-hidden="true" />
                {pt("Pause")}
              </Button>
            </div>
            <Button variant="secondary" onClick={() => {
              refresh();
              refreshExecutions();
              toast.success(pt("Automation detail refreshed"));
            }}
            >
              <RefreshCcw size={15} aria-hidden="true" />
              {pt("Refresh")}
            </Button>
            <Button disabled={executionBusy || archived} variant="secondary" onClick={handleExecute}>
              <Play size={15} aria-hidden="true" />
              {executionBusy ? pt("Running...") : pt("Run Now")}
            </Button>
            {archived ? (
              <Button disabled={archiveBusy} variant="secondary" onClick={handleRestore}>
                <StepForward size={15} aria-hidden="true" />
                {archiveBusy ? pt("Restoring...") : pt("Restore")}
              </Button>
            ) : (
              <Button disabled={archiveBusy} variant="secondary" onClick={handleArchive}>
                <Archive size={15} aria-hidden="true" />
                {archiveBusy ? pt("Archiving...") : pt("Archive")}
              </Button>
            )}
            {editing ? (
              <>
                <Button variant="secondary" onClick={resetDraft}>{pt("Cancel")}</Button>
                <Button disabled={!detailDirty || saving || archived || !selectedModelOptionCode} aria-busy={saving} onClick={() => void saveDetail()}>
                  <Save size={15} aria-hidden="true" />
                  {saving ? pt("Saving...") : pt("Save")}
                </Button>
              </>
            ) : (
              <Button disabled={archived} variant="secondary" onClick={() => setEditing(true)}>
                <Edit3 size={15} aria-hidden="true" />
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
              <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">Core definition</h2>
            </div>
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            <Field label="Automation name">
              <TextInput aria-label="Automation name" disabled={!editing || archived} value={name} onChange={(event) => setName(event.target.value)} />
            </Field>
            <Field label="Workspace">
              <TextInput aria-label="Workspace code" disabled value={automation.workspaceCode} />
            </Field>
            <Field label="Execution agent">
              <OptionSelect
                ariaLabel="Automation execution agent"
                disabled={!editing || archived || executionOptionsLoading}
                loading={executionOptionsLoading}
                loadingLabel="Loading agents..."
                options={executionAgentSelectOptions}
                value={selectedAgentCode}
                onChange={(value) => {
                  setSelectedAgentCode(value);
                  setSelectedModelOptionCode("");
                }}
              />
            </Field>
            <Field label="Execution model">
              <OptionSelect
                ariaLabel="Automation execution model"
                emptyLabel="No executable models"
                disabled={!editing || archived || executionOptionsLoading || !visibleModelOptions.length}
                loading={executionOptionsLoading}
                loadingLabel="Loading models..."
                options={executionModelSelectOptions}
                value={selectedModelOptionCode}
                onChange={setSelectedModelOptionCode}
              />
            </Field>
            <Field label="Cron expression">
              <TextInput aria-label="Cron expression" className="font-mono" disabled={!editing || archived} value={cronExpression} onChange={(event) => setCronExpression(event.target.value)} />
            </Field>
            <Field label="Scheduler expression">
              <TextInput aria-label="Scheduler expression" className="font-mono" disabled value={automation.schedulerExpression} />
            </Field>
          </div>
          <div className="mt-4">
            <Field label="Description">
              <TextArea
                aria-label="Automation description"
                className="min-h-24"
                disabled={!editing || archived}
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </Field>
          </div>
          <div className="mt-5 grid gap-1.5 border-t border-[var(--color-outline-variant)] pt-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Prompt body</span>
              <Button size="sm" type="button" variant="ghost" onClick={() => setPromptFullscreenOpen(true)}>
                <Maximize2 size={15} aria-hidden="true" />
                Fullscreen
              </Button>
            </div>
            {editing ? (
              <TextArea
                aria-label="Automation prompt"
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
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">Execution instances</h2>
            </div>
            <Button
              aria-busy={executionsLoading}
              disabled={executionsLoading}
              size="sm"
              type="button"
              variant="secondary"
              onClick={handleRefreshExecutions}
            >
              <RefreshCcw className={executionsLoading ? "animate-spin" : undefined} size={14} aria-hidden="true" />
              {pt("Refresh executions")}
            </Button>
          </div>
          {executionInitialLoading ? (
            <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]">
              <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                Loading execution history...
              </div>
              <div className="flex min-h-48 items-center justify-center px-5 py-8">
                <div className="h-1.5 w-32 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
                  <div className="h-full w-1/2 animate-pulse rounded-full bg-[var(--color-primary)]" />
                </div>
              </div>
            </div>
          ) : (
              <DataTable<AutomationExecutionListItem>
                variant="embedded"
                emptyState="No execution history has been recorded yet."
                rowKey={(row) => row.executionCode}
                rows={executionPageRecords}
                tableClassName="min-w-[980px]"
              pagination={(
                <TablePagination
                  endItem={executionBounds.end}
                  itemLabel="executions"
                  page={executionPageState.current}
                  pageCount={executionPageCount}
                  pageSize={executionPageState.size}
                  startItem={executionBounds.start}
                  totalItems={executionPageState.total}
                  onPageChange={(page) => setExecutionCurrent(Math.min(executionPageCount, Math.max(1, page)))}
                  onPageSizeChange={(pageSize) => {
                    setExecutionCurrent(1);
                    setExecutionSize(pageSize);
                  }}
                />
              )}
              columns={[
                {
                  header: "Execution",
                  id: "execution",
                  render: (row) => (
                    <div className="space-y-1">
                      <p className="font-mono text-[13px] text-[var(--color-on-surface)]">{row.executionCode}</p>
                      <p className="text-xs text-[var(--color-on-surface-variant)]">{formatTimestamp(row.createdAt)}</p>
                      <p className="text-xs text-[var(--color-on-surface-variant)]">
                        {row.sessionCode ?? pt("No linked session")}
                      </p>
                    </div>
                  ),
                },
                {
                  header: "Trigger",
                  id: "trigger",
                  render: (row) => (
                    <Badge tone={row.triggerSource === "Manual" ? "accent" : "neutral"}>
                      {row.triggerSource}
                    </Badge>
                  ),
                },
                {
                  header: "Status",
                  id: "status",
                  render: (row) => (
                    <Badge tone={getAutomationExecutionStatusTone(row.status)}>
                      {row.status}
                    </Badge>
                  ),
                },
                {
                  header: "Started",
                  id: "started",
                  render: (row) => formatTimestamp(row.startedAt),
                },
                {
                  header: "Finished",
                  id: "finished",
                  render: (row) => formatTimestamp(row.finishedAt),
                },
                {
                  header: "Summary",
                  id: "summary",
                  render: (row) => (
                    <span
                      className="line-clamp-4 max-w-[28rem] whitespace-pre-wrap text-sm leading-5 text-[var(--color-on-surface-variant)]"
                      title={row.summary ?? undefined}
                    >
                      {row.summary ?? "No summary"}
                    </span>
                  ),
                },
                {
                  header: "",
                  id: "actions",
                  align: "right",
                  render: (row) => {
                    const menuOpen = openExecutionRowMenu?.executionCode === row.executionCode;
                    const busy = executionRowBusyKey === `delete:${row.executionCode}`;

                    return (
                      <Button
                        aria-expanded={menuOpen}
                        aria-haspopup="menu"
                        aria-label={pt("Open actions for {{code}}", { code: row.executionCode })}
                        disabled={busy}
                        size="icon"
                        type="button"
                        variant="ghost"
                        onClick={(event) => {
                          event.stopPropagation();
                          toggleExecutionRowMenu(row, event.currentTarget);
                        }}
                      >
                        <MoreHorizontal size={16} aria-hidden="true" />
                      </Button>
                    );
                  },
                },
              ]}
            />
          )}
        </SurfaceCard>
      </div>
      {openExecutionRowMenu && openExecutionMenuRow
        ? createPortal(
          <div
            ref={executionRowMenuRef}
            style={{ left: openExecutionRowMenu.left, top: openExecutionRowMenu.top }}
            className="fixed z-[80] w-48 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={pt("Actions for {{code}}", { code: openExecutionMenuRow.executionCode })}
          >
            <button
              type="button"
              role="menuitem"
              className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={!openExecutionMenuRow.sessionCode}
              onClick={() => handleOpenExecutionSession(openExecutionMenuRow)}
            >
              <ExternalLink size={14} aria-hidden="true" />
              {pt("Open session")}
            </button>
            <button
              type="button"
              role="menuitem"
              className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={executionRowBusyKey === `delete:${openExecutionMenuRow.executionCode}`}
              onClick={() => requestDeleteExecution(openExecutionMenuRow)}
            >
              <Trash2 size={14} aria-hidden="true" />
              {pt("Delete")}
            </button>
          </div>,
          document.body,
        )
        : null}
      <ConfirmDialog
        ariaLabel={pt("Delete execution")}
        cancelText={pt("Cancel")}
        confirmText={executionRowBusyKey ? pt("Deleting...") : pt("Delete")}
        description={deleteTargetExecution
          ? pt("This will remove execution {{code}} from the history table.", { code: deleteTargetExecution.executionCode })
          : undefined}
        open={Boolean(deleteTargetExecution)}
        pending={Boolean(executionRowBusyKey)}
        title={pt("Delete execution")}
        onCancel={() => {
          if (!executionRowBusyKey) setDeleteTargetExecution(null);
        }}
        onConfirm={() => {
          void confirmDeleteExecution();
        }}
      />
      {promptFullscreenOpen ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={editing ? "Prompt fullscreen editor" : "Prompt fullscreen preview"}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Automation prompt</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {editing
                    ? "Edit the long-form prompt in a larger workspace."
                    : "Preview the rendered prompt that the scheduler will dispatch for this automation."}
                </p>
              </div>
              <Button size="icon" type="button" variant="ghost" aria-label="Close prompt fullscreen" onClick={() => setPromptFullscreenOpen(false)}>
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              {editing ? (
                <TextArea
                  aria-label="Automation prompt fullscreen"
                  className="h-full min-h-0 w-full resize-none"
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                />
              ) : (
                <div className="h-full overflow-y-auto pr-1">
                  <MarkdownContent ariaLabel="Automation prompt fullscreen preview" className="max-w-none" content={automation.prompt} />
                </div>
              )}
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
