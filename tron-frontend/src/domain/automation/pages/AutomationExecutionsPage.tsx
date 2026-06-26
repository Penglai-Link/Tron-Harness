import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  ExternalLink,
  MoreHorizontal,
  RefreshCcw,
  Trash2,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  ConfirmDialog,
  DataTable,
  DetailField,
  InspectorPanel,
  MarkdownContent,
  PageHeader,
  SurfaceCard,
  TablePagination,
  useDismissibleLayer,
  useConsoleShellConfig,
  useLocaleText,
  useOpenRightSidebar,
  useToast,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import {
  deleteAutomationExecution,
} from "@/domain/automation/api";
import {
  getAutomationExecutionStatusTone,
} from "@/domain/automation/constants";
import {
  useAutomationDetail,
  useAutomationExecutions,
} from "@/domain/automation/hooks";
import {
  deleteMockAutomationExecution,
} from "@/domain/automation/mock/data";
import type {
  AutomationExecutionListItem,
} from "@/domain/automation/model";
import {
  useChatMessageListBySession,
} from "@/domain/chat/hooks/useChatMessageListBySession";
import type {
  ChatMessage,
} from "@/domain/chat/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

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

function automationDetailHref(automationCode: string) {
  const params = new URLSearchParams({ automationCode });
  return `/automations/detail?${params.toString()}`;
}

function chatSessionDetailHref(sessionCode: string) {
  const params = new URLSearchParams({
    sessionCode,
    sourceTab: "automation",
  });
  return `/chats?${params.toString()}`;
}

function chatRoleLabel(role: ChatMessage["role"]) {
  return role === "ASSISTANT" ? "Assistant" : "User";
}

export function AutomationExecutionsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const pt = useLocaleText();
  const openRightSidebar = useOpenRightSidebar();
  const toast = useToast();
  const dataSource = getServiceDataSource("automation");
  const automationCode = searchParams.get("automationCode") ?? "";
  const { automation, refresh } = useAutomationDetail(automationCode);
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
  const executionPageCount = Math.max(1, Math.ceil(executionPageState.total / executionPageState.size));
  const executionBounds = executionPageBounds(
    executionPageState.current,
    executionPageState.size,
    executionPageState.total,
    executionPageRecords.length,
  );
  const executionInitialLoading = executionsLoading && executionPageState.total === 0;
  const [executionRowBusyKey, setExecutionRowBusyKey] = useState("");
  const [openExecutionRowMenu, setOpenExecutionRowMenu] = useState<ExecutionRowMenuState | null>(null);
  const [deleteTargetExecution, setDeleteTargetExecution] = useState<AutomationExecutionListItem | null>(null);
  const [selectedExecutionCode, setSelectedExecutionCode] = useState("");
  const executionRowMenuRef = useRef<HTMLDivElement | null>(null);
  const openExecutionMenuRow = openExecutionRowMenu
    ? executionPageRecords.find((item) => item.executionCode === openExecutionRowMenu.executionCode) ?? null
    : null;
  const selectedExecution = selectedExecutionCode
    ? executionPageRecords.find((item) => item.executionCode === selectedExecutionCode) ?? null
    : null;
  const selectedSessionCode = selectedExecution?.sessionCode ?? "";
  const transcriptMessages = useChatMessageListBySession(selectedSessionCode, 0, { suppressErrorToast: true });

  const rightPanel = useMemo(() => {
    if (!selectedExecution) return undefined;

    return (
      <div className="min-h-full px-5 py-5">
        <InspectorPanel title="Session detail">
          <div className="space-y-5">
            <div className="space-y-3">
              <DetailField label="Execution" value={selectedExecution.executionCode} />
              <DetailField label="Session" value={selectedExecution.sessionCode ?? pt("No linked session")} />
            </div>
            {selectedSessionCode ? (
              <>
                <div className="flex items-center justify-between gap-2 border-t border-[var(--color-outline-variant)] pt-4">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                    Conversation
                  </p>
                  <Button size="sm" type="button" variant="ghost" onClick={() => navigate(chatSessionDetailHref(selectedSessionCode))}>
                    <ExternalLink size={14} aria-hidden="true" />
                    {pt("Open session")}
                  </Button>
                </div>
                <div className="max-h-[calc(100vh-21rem)] space-y-3 overflow-y-auto pr-1">
                  {transcriptMessages.length ? transcriptMessages.map((message) => (
                    <article
                      className={cn(
                        "rounded-xl border p-3",
                        message.role === "ASSISTANT"
                          ? "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]"
                          : "border-[var(--color-primary)]/30 bg-[var(--color-primary-container)]/25",
                      )}
                      key={message.messageCode}
                    >
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="text-xs font-semibold text-[var(--color-on-surface)]">
                          {chatRoleLabel(message.role)}
                        </span>
                        <span className="truncate text-[11px] text-[var(--color-on-surface-variant)]" title={formatTimestamp(message.createdAt)}>
                          {formatTimestamp(message.createdAt)}
                        </span>
                      </div>
                      <MarkdownContent
                        ariaLabel={`${chatRoleLabel(message.role)} message`}
                        className="max-w-none text-sm leading-5"
                        content={message.content || "No content"}
                      />
                    </article>
                  )) : (
                    <div className="rounded-xl border border-dashed border-[var(--color-outline-variant)] p-4 text-sm text-[var(--color-on-surface-variant)]">
                      No messages found for this session.
                    </div>
                  )}
                </div>
              </>
            ) : (
              <div className="rounded-xl border border-dashed border-[var(--color-outline-variant)] p-4 text-sm text-[var(--color-on-surface-variant)]">
                This execution does not have a linked session.
              </div>
            )}
          </div>
        </InspectorPanel>
      </div>
    );
  }, [navigate, pt, selectedExecution, selectedSessionCode, transcriptMessages]);

  useConsoleShellConfig({ rightPanel });

  useEffect(() => {
    if (!selectedExecution) return;
    openRightSidebar();
  }, [openRightSidebar, selectedExecution]);

  useEffect(() => {
    setExecutionCurrent(1);
    setSelectedExecutionCode("");
  }, [automationCode]);

  useEffect(() => {
    setOpenExecutionRowMenu(null);
    setSelectedExecutionCode("");
  }, [automationCode, executionCurrent, executionSize]);

  useDismissibleLayer({
    onDismiss: () => setOpenExecutionRowMenu(null),
    open: openExecutionRowMenu !== null,
    popupRef: executionRowMenuRef,
    triggerElement: openExecutionRowMenu?.triggerElement,
  });

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

  function handleExecutionRowClick(row: AutomationExecutionListItem) {
    setOpenExecutionRowMenu(null);
    setSelectedExecutionCode(row.executionCode);
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
          { label: automation.automationCode || automationCode || pt("Detail"), href: automationDetailHref(automation.automationCode || automationCode) },
          { label: pt("Execution instances") },
        ]}
        title={pt("Execution instances")}
        actions={(
          <Button
            aria-busy={executionsLoading}
            disabled={executionsLoading}
            type="button"
            variant="secondary"
            onClick={handleRefreshExecutions}
          >
            <RefreshCcw className={executionsLoading ? "animate-spin" : undefined} size={15} aria-hidden="true" />
            {pt("Refresh")}
          </Button>
        )}
      />
      <SurfaceCard>
        <div className="mb-4 min-w-0">
          <h2 className="truncate text-lg font-semibold text-[var(--color-on-surface)]" title={automation.name || automationCode}>
            {automation.name || automationCode || pt("Automation Detail")}
          </h2>
          <p className="mt-1 truncate font-mono text-xs text-[var(--color-on-surface-variant)]" title={automation.automationCode || automationCode}>
            {automation.automationCode || automationCode}
          </p>
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
            onRowClick={handleExecutionRowClick}
            rowClassName={(row) => cn(
              selectedExecutionCode === row.executionCode
                ? "bg-[var(--color-primary-container)]/35 hover:bg-[var(--color-primary-container)]/45"
                : undefined,
            )}
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
                header: "Actions",
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
    </>
  );
}
