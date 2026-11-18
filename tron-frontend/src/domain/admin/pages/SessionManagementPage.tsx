import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Archive,
  FolderInput,
  LoaderCircle,
  RefreshCw,
  Square,
  Trash2,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable } from "@/components/data-table";
import { SearchField } from "@/components/search-field";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { TablePagination } from "@/components/table-pagination";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { PageHeader } from "@/shared";
import { getUserTimeZone } from "@/shared/lib/userTimeZone";
import { fetchWorkspaceOptions } from "@/domain/admin/api";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";
import { fetchProjects } from "@/domain/project";
import type { ProjectRecord } from "@/domain/project";
import {
  archiveManagedChatSessions,
  deleteManagedChatSessions,
  fetchManagedChatSessions,
  moveManagedChatSessions,
  stopManagedChatSessions,
  type ManagedChatSession,
  type ManagedSessionBatchItem,
  type ManagedSessionBatchResult,
} from "@/domain/admin/api/sessionManagementApi";

const PAGE_SIZE = 25;

type StatusFilter = "ALL" | "ACTIVE" | "ARCHIVED";
type SourceFilter = "ALL" | "MANUAL" | "TASK" | "AUTOMATION";

function getErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function displayTimestamp(value: number) {
  return new Date(value).toLocaleString(
    document.documentElement.lang || undefined,
    { timeZone: getUserTimeZone() },
  );
}

export function SessionManagementPage() {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const [workspaceOptions, setWorkspaceOptions] = useState<WorkspaceOption[]>([]);
  const [workspaceCode, setWorkspaceCode] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("ALL");
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>("ALL");
  const [search, setSearch] = useState("");
  const [sessions, setSessions] = useState<ManagedChatSession[]>([]);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState(1);
  const [pageSize, setPageSize] = useState(PAGE_SIZE);
  const [selectedCodes, setSelectedCodes] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [stopPending, setStopPending] = useState<string[] | null>(null);
  const [archivePending, setArchivePending] = useState<string[] | null>(null);
  const [deletePending, setDeletePending] = useState<string[] | null>(null);
  const [moveOpen, setMoveOpen] = useState(false);
  const [moveWorkspace, setMoveWorkspace] = useState("");
  const [moveProject, setMoveProject] = useState("");
  const [moveProjects, setMoveProjects] = useState<ProjectRecord[]>([]);
  const [moveLoading, setMoveLoading] = useState(false);
  const [moveError, setMoveError] = useState("");
  const [failureItems, setFailureItems] = useState<ManagedSessionBatchItem[]>([]);
  const listRequest = useRef(0);

  const workspaceLabels = useMemo(
    () => new Map(workspaceOptions.map((workspace) => [workspace.workspaceCode, workspace.label || workspace.name])),
    [workspaceOptions],
  );

  useEffect(() => {
    let cancelled = false;
    void fetchWorkspaceOptions({ includeArchived: false }).then((options) => {
      if (!cancelled) setWorkspaceOptions(options);
    }).catch(() => {
      if (!cancelled) toast.error(getErrorMessage(null, t("sessionManagement.loadFailed")));
    });
    return () => { cancelled = true; };
  }, [toast, t]);

  const loadSessions = useCallback(async (page: number) => {
    const generation = ++listRequest.current;
    setLoading(true);
    try {
      const query = {
        workspaceCode: workspaceCode || undefined,
        status: statusFilter === "ALL" ? undefined : statusFilter,
        sourceType: sourceFilter === "ALL" ? undefined : sourceFilter,
        search: search || undefined,
        current: page,
        size: pageSize,
      };
      let result = await fetchManagedChatSessions(query);
      const lastPage = Math.max(1, Math.ceil(result.total / pageSize));
      if (page > lastPage) {
        result = await fetchManagedChatSessions({ ...query, current: lastPage });
      }
      if (generation !== listRequest.current) return;
      setSessions(result.records);
      setTotal(result.total);
      setCurrent(result.current);
      setSelectedCodes(new Set());
    } catch (error) {
      if (generation === listRequest.current) toast.error(getErrorMessage(error, t("sessionManagement.loadFailed")));
    } finally {
      if (generation === listRequest.current) setLoading(false);
    }
  }, [toast, t, workspaceCode, statusFilter, sourceFilter, search, pageSize]);

  useEffect(() => {
    setCurrent(1);
    setSelectedCodes(new Set());
    setFailureItems([]);
    void loadSessions(1);
    return () => { ++listRequest.current; };
  }, [loadSessions]);

  const toggleSelected = (sessionCode: string) => {
    setSelectedCodes((previous) => {
      const next = new Set(previous);
      if (next.has(sessionCode)) next.delete(sessionCode);
      else next.add(sessionCode);
      return next;
    });
  };

  const toggleAll = () => {
    setSelectedCodes((previous) => previous.size === sessions.length
      ? new Set()
      : new Set(sessions.map((session) => session.sessionCode)));
  };

  const applyBatchResult = (result: ManagedSessionBatchResult) => {
    const failures = result.items.filter((item) => !item.success);
    setFailureItems(failures);
    setSelectedCodes(new Set(failures.map((item) => item.sessionCode)));
    if (result.failed === 0) {
      toast.success(t("sessionManagement.batchSucceeded", { count: result.succeeded }));
    } else {
      toast.error(t("sessionManagement.batchWithFailures", { succeeded: result.succeeded, failed: result.failed }));
    }
  };

  const confirmStop = async () => {
    const codes = stopPending;
    if (!codes || codes.length === 0) return;
    setBusy(true);
    try {
      await stopManagedChatSessions(codes);
      toast.success(t("sessionManagement.stopped", { count: codes.length }));
      await loadSessions(current);
    } catch (error) {
      toast.error(getErrorMessage(error, t("sessionManagement.stopFailed")));
    } finally {
      setBusy(false);
      setStopPending(null);
    }
  };

  const confirmArchive = async () => {
    const codes = archivePending;
    if (!codes || codes.length === 0) return;
    setBusy(true);
    try {
      const result = await archiveManagedChatSessions(codes);
      applyBatchResult(result);
      await loadSessions(current);
    } catch (error) {
      toast.error(getErrorMessage(error, t("sessionManagement.archiveFailed")));
    } finally {
      setBusy(false);
      setArchivePending(null);
    }
  };

  const confirmDelete = async () => {
    const codes = deletePending;
    if (!codes || codes.length === 0) return;
    setBusy(true);
    try {
      const result = await deleteManagedChatSessions(codes);
      applyBatchResult(result);
      await loadSessions(current);
    } catch (error) {
      toast.error(getErrorMessage(error, t("sessionManagement.deleteFailed")));
    } finally {
      setBusy(false);
      setDeletePending(null);
    }
  };

  const openMoveDialog = () => {
    setMoveWorkspace(workspaceCode || "");
    setMoveProject("");
    setMoveProjects([]);
    setMoveError("");
    setMoveOpen(true);
  };

  useEffect(() => {
    if (!moveOpen) return;
    let live = true;
    if (!moveWorkspace) {
      setMoveProjects([]);
      return;
    }
    setMoveLoading(true);
    setMoveError("");
    fetchProjects(moveWorkspace).then((projects) => {
      if (live) setMoveProjects(projects);
    }).catch((error: unknown) => {
      if (live) setMoveError(getErrorMessage(error, t("sessionManagement.moveTopicsFailed")));
    }).finally(() => {
      if (live) setMoveLoading(false);
    });
    return () => { live = false; };
  }, [moveOpen, moveWorkspace, t]);

  const confirmMove = async () => {
    const codes = [...selectedCodes];
    if (codes.length === 0 || !moveWorkspace || busy) return;
    setBusy(true);
    try {
      const result = await moveManagedChatSessions({
        sessionCodes: codes,
        targetWorkspaceCode: moveWorkspace,
        targetProjectCode: moveProject || null,
      });
      applyBatchResult(result);
      setMoveOpen(false);
      await loadSessions(1);
    } catch (error) {
      toast.error(getErrorMessage(error, t("sessionManagement.moveFailed")));
    } finally {
      setBusy(false);
    }
  };

  const nativeErrorLabel = (errorCode: string | null) => {
    if (!errorCode) return t("sessionManagement.operationFailed");
    return t(`sessionManagement.errors.${errorCode}`, { defaultValue: t("sessionManagement.operationFailed") });
  };

  const allSelected = sessions.length > 0 && selectedCodes.size === sessions.length;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const selectedRunning = sessions.filter((session) => selectedCodes.has(session.sessionCode) && session.running);
  const selectedArray = useMemo(() => [...selectedCodes], [selectedCodes]);
  const movableCount = selectedArray.length;

  return (
    <>
      <PageHeader title={t("sessionManagement.title")} />
      <div className="space-y-4">
        <DataTable<ManagedChatSession>
          rows={loading ? [] : sessions}
          rowKey={(session) => session.sessionCode}
          tableClassName="table-fixed min-w-[900px]"
          isRowSelected={(session) => selectedCodes.has(session.sessionCode)}
          emptyState={t(loading ? "sessionManagement.loading" : "sessionManagement.empty")}
          toolbar={(
            <>
              <SelectInput
                aria-label={t("sessionManagement.workspace")}
                className="w-full sm:w-52"
                value={workspaceCode || "__all__"}
                disabled={busy}
                onValueChange={(value) => setWorkspaceCode(value === "__all__" ? "" : String(value))}
              >
                <SelectItem value="__all__">{t("sessionManagement.allWorkspaces")}</SelectItem>
                {workspaceOptions.map((workspace) => <SelectItem key={workspace.workspaceCode} value={workspace.workspaceCode}>{workspace.label || workspace.name}</SelectItem>)}
              </SelectInput>
              <SelectInput
                aria-label={t("sessionManagement.status")}
                className="w-full sm:w-36"
                value={statusFilter}
                disabled={busy}
                onValueChange={(value) => { if (value === "ALL" || value === "ACTIVE" || value === "ARCHIVED") setStatusFilter(value); }}
              >
                <SelectItem value="ALL">{t("sessionManagement.statusAll")}</SelectItem>
                <SelectItem value="ACTIVE">{t("sessionManagement.statusActive")}</SelectItem>
                <SelectItem value="ARCHIVED">{t("sessionManagement.statusArchived")}</SelectItem>
              </SelectInput>
              <SelectInput
                aria-label={t("sessionManagement.source")}
                className="w-full sm:w-36"
                value={sourceFilter}
                disabled={busy}
                onValueChange={(value) => { if (value === "ALL" || value === "MANUAL" || value === "TASK" || value === "AUTOMATION") setSourceFilter(value); }}
              >
                <SelectItem value="ALL">{t("sessionManagement.sourceAll")}</SelectItem>
                <SelectItem value="MANUAL">{t("sessionManagement.sourceManual")}</SelectItem>
                <SelectItem value="TASK">{t("sessionManagement.sourceTask")}</SelectItem>
                <SelectItem value="AUTOMATION">{t("sessionManagement.sourceAutomation")}</SelectItem>
              </SelectInput>
              <SearchField
                className="w-full sm:w-56"
                placeholder={t("sessionManagement.searchPlaceholder")}
                value={search}
                onChange={setSearch}
              />
              <div className="ml-auto flex items-center gap-2">
                {selectedCodes.size > 0 && (
                  <Button variant="secondary" disabled={busy || selectedRunning.length === 0} onClick={() => setStopPending(selectedRunning.map((session) => session.sessionCode))}>
                    {t("sessionManagement.stopSelected", { count: selectedRunning.length })}
                  </Button>
                )}
                {selectedCodes.size > 0 && (
                  <Button variant="outline" disabled={busy || movableCount === 0} onClick={openMoveDialog}>
                    {t("sessionManagement.moveSelected", { count: movableCount })}
                  </Button>
                )}
                {selectedCodes.size > 0 && (
                  <Button variant="secondary" disabled={busy} onClick={() => setArchivePending(selectedArray)}>
                    {t("sessionManagement.archiveSelected", { count: selectedCodes.size })}
                  </Button>
                )}
                {selectedCodes.size > 0 && (
                  <Button variant="destructive" disabled={busy} onClick={() => setDeletePending(selectedArray)}>
                    {t("sessionManagement.deleteSelected", { count: selectedCodes.size })}
                  </Button>
                )}
                <Button variant="outline" size="sm" disabled={loading || busy} onClick={() => void loadSessions(current)}>
                  {loading ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : <RefreshCw aria-hidden="true" />}
                  {t("sessionManagement.refresh")}
                </Button>
              </div>
            </>
          )}
          pagination={<TablePagination
            itemLabel={t("sessionManagement.itemLabel")}
            totalItems={total}
            startItem={total === 0 ? 0 : (current - 1) * pageSize + 1}
            endItem={Math.min(current * pageSize, total)}
            page={current}
            pageCount={pageCount}
            pageSize={pageSize}
            onPageChange={loading || busy ? undefined : (page) => void loadSessions(Math.max(1, Math.min(page, pageCount)))}
            onPageSizeChange={loading || busy ? undefined : setPageSize}
          />}
          columns={[
            { id: "selection", width: "56px", header: <input type="checkbox" aria-label={t("sessionManagement.selectPage")} checked={allSelected} disabled={loading || busy || sessions.length === 0} onChange={toggleAll} />, render: (session) => <input type="checkbox" aria-label={t("sessionManagement.selectSession", { title: session.title })} checked={selectedCodes.has(session.sessionCode)} disabled={busy} onChange={() => toggleSelected(session.sessionCode)} /> },
            {
              id: "session", header: t("sessionManagement.session"), cellClassName: "min-w-0", render: (session) => (
                <div className="min-w-0">
                  <p className="truncate font-medium" title={session.title}>{session.title}</p>
                  <p className="truncate text-xs text-[var(--color-on-surface-variant)]" title={session.sessionCode}>
                    {session.sessionCode}
                  </p>
                </div>
              ),
            },
            {
              id: "workspace", header: t("sessionManagement.workspace"), width: "160px", render: (session) => {
                const workspace = workspaceLabels.get(session.workspaceCode) ?? session.workspaceCode;
                return (
                  <div className="min-w-0">
                    <p className="truncate" title={workspace}>{workspace}</p>
                    {session.projectCode
                      ? <p className="truncate text-xs text-[var(--color-on-surface-variant)]" title={session.projectCode}>{t("sessionManagement.inTopic")}</p>
                      : <p className="truncate text-xs text-[var(--color-on-surface-variant)]">{t("sessionManagement.quickChat")}</p>}
                  </div>
                );
              },
            },
            {
              id: "source", header: t("sessionManagement.source"), width: "110px", render: (session) => (
                <Badge variant="secondary">{t(`sessionManagement.sources.${session.sourceType}`, { defaultValue: session.sourceType })}</Badge>
              ),
            },
            {
              id: "status", header: t("sessionManagement.status"), width: "130px", render: (session) => (
                <span className="inline-flex items-center gap-1.5">
                  {session.running
                    ? <LoaderCircle className="animate-spin text-[var(--color-focus-ring)]" size={14} aria-label={t("sessionManagement.running")} />
                    : null}
                  <Badge variant={session.status === "ARCHIVED" ? "warning" : session.running ? "info" : "success"}>
                    {t(session.status === "ARCHIVED" ? "sessionManagement.statusArchived" : "sessionManagement.statusActive")}
                  </Badge>
                </span>
              ),
            },
            { id: "updatedAt", header: t("sessionManagement.updatedAt"), width: "200px", render: (session) => <time className="text-xs">{displayTimestamp(session.updatedAt)}</time> },
            {
              id: "actions", header: t("sessionManagement.actions"), width: "130px", align: "right", headerClassName: "sticky right-0 z-20 border-l border-[var(--color-outline-variant)] bg-[var(--tron-table-header-bg,var(--color-elevated-surface))]", cellClassName: "sticky right-0 z-10 border-l border-[var(--color-outline-variant)] bg-[var(-- _tron-row-bg)]", render: (session) => <div className="flex justify-end gap-1">
                <Button variant="ghost" size="icon" title={t("sessionManagement.stop")} aria-label={t("sessionManagement.stop")} disabled={busy || !session.running} onClick={() => setStopPending([session.sessionCode])}><Square /></Button>
                <Button variant="ghost" size="icon" title={t("sessionManagement.archive")} aria-label={t("sessionManagement.archive")} disabled={busy || session.status === "ARCHIVED"} onClick={() => setArchivePending([session.sessionCode])}><Archive /></Button>
                <Button variant="ghost" size="icon" className="text-[var(--color-error)]" title={t("sessionManagement.delete")} aria-label={t("sessionManagement.delete")} disabled={busy} onClick={() => setDeletePending([session.sessionCode])}><Trash2 /></Button>
              </div>,
            },
          ]}
        />
        {failureItems.length > 0 && (
          <ul className="space-y-1 rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error-container)]/30 p-3 text-xs" aria-live="polite">
            {failureItems.map((item) => (
              <li key={item.sessionCode}>
                <span className="font-mono">{item.sessionCode}</span>: {nativeErrorLabel(item.errorCode)}
              </li>
            ))}
          </ul>
        )}
      </div>

      <Dialog open={moveOpen} onOpenChange={(open) => { if (!open && !busy) setMoveOpen(false); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t("sessionManagement.moveTitle", { count: movableCount })}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">{t("sessionManagement.moveDescription")}</p>
          <div className="space-y-3">
            <div>
              <p className="mb-1 text-xs font-medium">{t("sessionManagement.targetWorkspace")}</p>
              <SelectInput
                aria-label={t("sessionManagement.targetWorkspace")}
                value={moveWorkspace || "__all__"}
                disabled={busy}
                onValueChange={(value) => { setMoveWorkspace(value === "__all__" ? "" : String(value)); setMoveProject(""); }}
              >
                <SelectItem value="__all__">{t("sessionManagement.selectWorkspace")}</SelectItem>
                {workspaceOptions.map((workspace) => <SelectItem key={workspace.workspaceCode} value={workspace.workspaceCode}>{workspace.label || workspace.name}</SelectItem>)}
              </SelectInput>
            </div>
            <div>
              <p className="mb-1 text-xs font-medium">{t("sessionManagement.targetTopic")}</p>
              {moveLoading ? <p className="text-sm text-muted-foreground">{t("sessionManagement.loading")}</p> : (
                <SelectInput
                  aria-label={t("sessionManagement.targetTopic")}
                  value={moveProject || "__quick__"
                  }
                  disabled={busy || !moveWorkspace}
                  onValueChange={(value) => setMoveProject(value === "__quick__" ? "" : String(value))}
                >
                  <SelectItem value="__quick__">{t("sessionManagement.quickChats")}</SelectItem>
                  {moveProjects.map((project) => <SelectItem key={project.projectCode} value={project.projectCode}>{project.name}</SelectItem>)}
                </SelectInput>
              )}
            </div>
            {moveError ? <p role="alert" className="text-sm text-destructive">{moveError}</p> : null}
          </div>
          <DialogFooter>
            <Button variant="outline" disabled={busy} onClick={() => setMoveOpen(false)}>{t("sessionManagement.cancel")}</Button>
            <Button disabled={busy || !moveWorkspace} onClick={() => void confirmMove()}>
              {busy ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : <FolderInput aria-hidden="true" />}
              {t("sessionManagement.moveConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={stopPending !== null}
        title={t("sessionManagement.stopTitle", { count: stopPending?.length ?? 0 })}
        description={t("sessionManagement.confirmStop", { count: stopPending?.length ?? 0 })}
        cancelText={t("sessionManagement.cancel")}
        confirmText={t("sessionManagement.stop")}
        pending={busy}
        onCancel={() => setStopPending(null)}
        onConfirm={() => void confirmStop()}
      />
      <ConfirmDialog
        open={archivePending !== null}
        title={t("sessionManagement.archiveTitle", { count: archivePending?.length ?? 0 })}
        description={t("sessionManagement.confirmArchive", { count: archivePending?.length ?? 0 })}
        cancelText={t("sessionManagement.cancel")}
        confirmText={t("sessionManagement.archive")}
        pending={busy}
        onCancel={() => setArchivePending(null)}
        onConfirm={() => void confirmArchive()}
      />
      <ConfirmDialog
        tone="danger"
        open={deletePending !== null}
        title={t("sessionManagement.deleteTitle", { count: deletePending?.length ?? 0 })}
        description={t("sessionManagement.confirmDelete", { count: deletePending?.length ?? 0 })}
        cancelText={t("sessionManagement.cancel")}
        confirmText={t("sessionManagement.delete")}
        pending={busy}
        onCancel={() => setDeletePending(null)}
        onConfirm={() => void confirmDelete()}
      />
    </>
  );
}
