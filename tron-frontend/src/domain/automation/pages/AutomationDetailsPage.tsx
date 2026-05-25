import {
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  useSearchParams,
} from "react-router";
import {
  Archive,
  Edit3,
  Maximize2,
  Pause,
  Play,
  RefreshCcw,
  Save,
  StepForward,
  X,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  PageHeader,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  useToast,
} from "@/shared";
import { cn } from "@/shared/lib/utils";

import {
  archiveAutomation,
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
  executeMockAutomation,
  restoreMockAutomation,
  updateMockAutomation,
  updateMockAutomationStatus,
} from "@/domain/automation/mock/data";
import type {
  AutomationExecutionListItem,
  AutomationOperationalStatus,
} from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

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

function MarkdownContent({
  ariaLabel,
  className,
  content,
}: {
  ariaLabel?: string;
  className?: string;
  content?: string | null;
}) {
  const value = content?.trim();

  if (!value) {
    return (
      <div aria-label={ariaLabel} className={cn("min-h-6 py-1 text-sm leading-6 text-[var(--color-on-surface)]", className)}>
        <span className="text-[var(--color-on-surface-variant)]">None</span>
      </div>
    );
  }

  return (
    <div
      aria-label={ariaLabel}
      className={cn(
        "min-h-6 py-1 text-sm leading-7 text-[var(--color-on-surface)]",
        "[&_a]:text-[var(--color-primary)] [&_a]:underline-offset-2 hover:[&_a]:underline",
        "[&_blockquote]:mb-3 [&_blockquote]:border-l-2 [&_blockquote]:border-[var(--color-outline-variant)] [&_blockquote]:pl-4 [&_blockquote]:text-[var(--color-on-surface-variant)]",
        "[&_code]:font-mono [&_code]:text-[13px]",
        "[&_h1]:mb-4 [&_h1]:text-2xl [&_h1]:font-semibold [&_h1]:leading-tight",
        "[&_h2]:mb-3 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:leading-tight",
        "[&_h3]:mb-2 [&_h3]:text-lg [&_h3]:font-semibold [&_h3]:leading-tight",
        "[&_hr]:my-4 [&_hr]:border-[var(--color-outline-variant)]",
        "[&_li]:mb-1",
        "[&_ol]:mb-3 [&_ol]:list-decimal [&_ol]:pl-5",
        "[&_p]:mb-3 [&_p:last-child]:mb-0",
        "[&_p_code]:rounded [&_p_code]:bg-[var(--color-surface-container-lowest)] [&_p_code]:px-1.5 [&_p_code]:py-0.5",
        "[&_pre]:mb-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-[var(--color-surface-container-lowest)] [&_pre]:p-3",
        "[&_pre_code]:bg-transparent [&_pre_code]:p-0",
        "[&_table]:mb-3 [&_table]:w-full [&_table]:border-collapse [&_table]:overflow-hidden [&_table]:rounded-lg",
        "[&_tbody_tr]:border-b [&_tbody_tr]:border-[var(--color-outline-variant)] [&_tbody_tr:last-child]:border-b-0",
        "[&_td]:px-3 [&_td]:py-2 [&_td]:align-top",
        "[&_th]:border-b [&_th]:border-[var(--color-outline-variant)] [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:font-semibold",
        "[&_ul]:mb-3 [&_ul]:list-disc [&_ul]:pl-5",
        className,
      )}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{value}</ReactMarkdown>
    </div>
  );
}

function DetailField({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <div className="mt-1">{value}</div>
    </div>
  );
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

export function AutomationDetailsPage() {
  const [searchParams] = useSearchParams();
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
  const automationRef = useRef(automation);
  automationRef.current = automation;
  const archived = automation.status === "Archived" || Boolean(automation.archivedAt);

  useEffect(() => {
    setName(automation.name);
    setDescription(automation.description ?? "");
    setCronExpression(automation.cronExpression);
    setPrompt(automation.prompt);
    setEditing(false);
    setPromptFullscreenOpen(false);
  }, [automation]);

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

  const detailDirty = name !== automation.name
    || description !== (automation.description ?? "")
    || cronExpression !== automation.cronExpression
    || prompt !== automation.prompt;
  const executionPageCount = Math.max(1, Math.ceil(executionPageState.total / executionPageState.size));
  const executionBounds = executionPageBounds(
    executionPageState.current,
    executionPageState.size,
    executionPageState.total,
    executionPageState.records.length,
  );
  const executionInitialLoading = executionsLoading && executionPageState.total === 0;

  function resetDraft() {
    setName(automation.name);
    setDescription(automation.description ?? "");
    setCronExpression(automation.cronExpression);
    setPrompt(automation.prompt);
    setPromptFullscreenOpen(false);
    setEditing(false);
  }

  async function saveDetail() {
    if (saving || archived || !detailDirty) return;
    setSaving(true);

    try {
      const payload = {
        cronExpression,
        description,
        name,
        prompt,
      };
      const updated = dataSource === "api"
        ? await updateAutomation(automation.automationCode, payload)
        : updateMockAutomation(automation.automationCode, payload);
      setAutomation(updated);
      setEditing(false);
      setPromptFullscreenOpen(false);
      toast.success(`Automation ${updated.automationCode} saved`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation could not be saved");
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
      toast.success(`Automation ${updated.automationCode} ${status === "Active" ? "activated" : "paused"}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation status could not be updated");
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
      toast.success(`Automation ${lifecycle.automationCode} archived`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation could not be archived");
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
      toast.success(`Automation ${lifecycle.automationCode} restored to ${lifecycle.status}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation could not be restored");
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
      toast.success(`Manual execution recorded for ${updated.automationCode}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Automation execution could not be recorded");
    } finally {
      setExecutionBusy(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: "Automations", href: "/automations" },
          { label: automation.automationCode || "Detail" },
        ]}
        title={automation.name || "Automation Detail"}
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
                激活
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
                暂停
              </Button>
            </div>
            <Button variant="secondary" onClick={() => {
              refresh();
              refreshExecutions();
              toast.success("Automation detail refreshed");
            }}
            >
              <RefreshCcw size={15} aria-hidden="true" />
              Refresh
            </Button>
            <Button disabled={executionBusy || archived} variant="secondary" onClick={handleExecute}>
              <Play size={15} aria-hidden="true" />
              {executionBusy ? "Running..." : "Run Now"}
            </Button>
            {archived ? (
              <Button disabled={archiveBusy} variant="secondary" onClick={handleRestore}>
                <StepForward size={15} aria-hidden="true" />
                {archiveBusy ? "Restoring..." : "Restore"}
              </Button>
            ) : (
              <Button disabled={archiveBusy} variant="secondary" onClick={handleArchive}>
                <Archive size={15} aria-hidden="true" />
                {archiveBusy ? "Archiving..." : "Archive"}
              </Button>
            )}
            {editing ? (
              <>
                <Button variant="secondary" onClick={resetDraft}>Cancel</Button>
                <Button disabled={!detailDirty || saving || archived} aria-busy={saving} onClick={() => void saveDetail()}>
                  <Save size={15} aria-hidden="true" />
                  {saving ? "Saving..." : "Save"}
                </Button>
              </>
            ) : (
              <Button disabled={archived} variant="secondary" onClick={() => setEditing(true)}>
                <Edit3 size={15} aria-hidden="true" />
                Edit
              </Button>
            )}
          </>
        )}
      />
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
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
            <div className="mb-4">
              <div>
                <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">Execution instances</h2>
              </div>
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
                emptyState="No execution history has been recorded yet."
                rowKey={(row) => row.executionCode}
                rows={executionPageState.records}
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
                      <span className="max-w-[28rem] whitespace-pre-wrap text-sm text-[var(--color-on-surface-variant)]">
                        {row.summary ?? "No summary"}
                      </span>
                    ),
                  },
                ]}
              />
            )}
          </SurfaceCard>
        </div>
        <div className="space-y-5">
          <SurfaceCard className="space-y-4">
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
          </SurfaceCard>
          <SurfaceCard className="space-y-4">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Metadata</p>
            <DetailField label="Automation code" value={<ReadValue>{automation.automationCode}</ReadValue>} />
            <DetailField label="Timezone" value={<ReadValue>{automation.timezone}</ReadValue>} />
            <DetailField label="Created" value={formatTimestamp(automation.createdAt)} />
            <DetailField label="Updated" value={formatTimestamp(automation.updatedAt)} />
            <DetailField label="Archived at" value={formatTimestamp(automation.archivedAt)} />
            <DetailField label="Previous status" value={<ReadValue>{automation.previousStatus}</ReadValue>} />
          </SurfaceCard>
        </div>
      </div>
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
