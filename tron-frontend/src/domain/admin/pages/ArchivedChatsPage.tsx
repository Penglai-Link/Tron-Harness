import { getUserTimeZone } from "@/shared/lib/userTimeZone";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChatA2uiSession } from "@eliteforge/agent-ui-chat-core";
import { ChatA2UI } from "@eliteforge/agent-ui-chat-react";

import { Button } from "@/components/ui/button";
import { PageHeader } from "@/shared";
import { DataTable } from "@/components/data-table";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { TablePagination } from "@/components/table-pagination";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { fetchWorkspaceOptions } from "@/domain/admin/api";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";
import type { ArchivedMessage, ArchivedSession, ArchivedSessionDetail } from "@/domain/admin/api/archivedChatsApi";
import {
  deleteArchivedChatSessions,
  fetchArchivedChatDetail,
  downloadArchivedChatAttachment,
  fetchArchivedChatSessions,
  restoreArchivedChatSession,
  type ArchivedChatSessionDeleteItem,
} from "@/domain/admin/api/archivedChatsApi";
import { useToast } from "@/components/ui/toast";

const PAGE_SIZE = 25;

function getErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function displayTimestamp(value: number) {
  return new Date(value).toLocaleString(document.documentElement.lang || undefined, { timeZone: getUserTimeZone() });
}

export function ArchivedChatsPage() {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const [workspaceOptions, setWorkspaceOptions] = useState<WorkspaceOption[]>([]);
  const [workspaceCode, setWorkspaceCode] = useState("");
  const [sessions, setSessions] = useState<ArchivedSession[]>([]);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState(1);
  const [pageSize, setPageSize] = useState(PAGE_SIZE);
  const [selectedCodes, setSelectedCodes] = useState<Set<string>>(new Set());
  const [failedCodes, setFailedCodes] = useState<string[]>([]);
  const [deleteErrors, setDeleteErrors] = useState<ArchivedChatSessionDeleteItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [deleteCodes, setDeleteCodes] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [viewing, setViewing] = useState<ArchivedSession | null>(null);
  const [messages, setMessages] = useState<ArchivedMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [detail, setDetail] = useState<ArchivedSessionDetail | null>(null);
  const archivedA2ui = useMemo(() => {
    if (!detail?.a2uiState) return undefined;
    const runtime = new ChatA2uiSession(detail.session.sessionCode);
    runtime.hydrate(detail.a2uiState);
    return runtime;
  }, [detail]);
  const listRequest = useRef(0);
  const detailRequest = useRef(0);

  const workspaceLabels = useMemo(
    () => new Map(workspaceOptions.map((workspace) => [workspace.workspaceCode, workspace.label || workspace.name])),
    [workspaceOptions],
  );

  const loadSessions = useCallback(async (page: number) => {
    const generation = ++listRequest.current;
    setLoading(true);
    try {
      const query = {
        workspaceCode: workspaceCode || undefined,
        current: page,
        size: pageSize,
      };
      let result = await fetchArchivedChatSessions(query);
      const lastPage = Math.max(1, Math.ceil(result.total / pageSize));
      if (page > lastPage) {
        result = await fetchArchivedChatSessions({ ...query, current: lastPage });
      }
      if (generation !== listRequest.current) return;
      setSessions(result.records);
      setTotal(result.total);
      setCurrent(result.current);
      setSelectedCodes(new Set());
    } catch (error) {
      if (generation === listRequest.current) toast.error(getErrorMessage(error, t("pages:archivedChats.loadFailed")));
    } finally {
      if (generation === listRequest.current) setLoading(false);
    }
  }, [toast, t, workspaceCode, pageSize]);

  useEffect(() => {
    let cancelled = false;
    void fetchWorkspaceOptions({ includeArchived: true }).then((options) => {
      if (!cancelled) setWorkspaceOptions(options);
    }).catch((error: unknown) => {
      if (!cancelled) toast.error(getErrorMessage(error, t("pages:archivedChats.workspacesFailed")));
    });
    return () => { cancelled = true; };
  }, [toast, t]);

  useEffect(() => {
    setCurrent(1);
    setSelectedCodes(new Set()); setFailedCodes([]); setDeleteErrors([]);
    ++detailRequest.current; setViewing(null); setDetail(null);
    void loadSessions(1);
    return () => { ++listRequest.current; ++detailRequest.current; };
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

  const openSession = async (session: ArchivedSession) => {
    const generation = ++detailRequest.current;
    setViewing(session);
    setDetail(null);
    setMessages([]);
    setMessagesLoading(true);
    try {
      const result = await fetchArchivedChatDetail(session.sessionCode);
      if (generation !== detailRequest.current) return;
      setDetail(result); setMessages(result.messages);
    } catch (error) {
      if (generation === detailRequest.current) toast.error(getErrorMessage(error, t("pages:archivedChats.viewFailed")));
    } finally {
      if (generation === detailRequest.current) setMessagesLoading(false);
    }
  };

  const restoreSession = async (session: ArchivedSession) => {
    setBusy(true);
    try {
      await restoreArchivedChatSession(session.sessionCode);
      window.dispatchEvent(new CustomEvent("tron:chat-session-updated", { detail: { workspaceCode: session.workspaceCode, sessionCode: session.sessionCode } }));
      if (viewing?.sessionCode === session.sessionCode) { ++detailRequest.current; setViewing(null); setDetail(null); }
      toast.success(t("pages:archivedChats.restored"));
      setFailedCodes((codes) => codes.filter((code) => code !== session.sessionCode));
      setDeleteErrors((items) => items.filter((item) => item.sessionCode !== session.sessionCode));
      await loadSessions(current);
    } catch (error) {
      toast.error(getErrorMessage(error, t("pages:archivedChats.restoreFailed")));
    } finally {
      setBusy(false);
    }
  };

  const deleteSelected = (codes: string[]) => setDeleteCodes(codes);

  const confirmDelete = async () => {
    const codes = deleteCodes;
    if (codes.length === 0) return;
    setBusy(true);
    try {
      const result = await deleteArchivedChatSessions(codes);
      if (viewing && result.items.some((item) => item.success && item.sessionCode === viewing.sessionCode)) {
        ++detailRequest.current; setViewing(null); setDetail(null);
      }
      const failures = result.items.filter((item: ArchivedChatSessionDeleteItem) => !item.success);
      const failed = failures.map((item) => item.sessionCode);
      setFailedCodes(failed);
      setDeleteErrors(failures);
      setSelectedCodes(new Set(failed));
      if (result.failed === 0) {
        toast.success(t("pages:archivedChats.deleted", { count: result.succeeded }));
      } else {
        toast.error(t("pages:archivedChats.deletedWithFailures", { succeeded: result.succeeded, failed: result.failed }));
      }
      await loadSessions(current);
      setFailedCodes(failed);
      setSelectedCodes(new Set(failed));
    } catch (error) {
      toast.error(getErrorMessage(error, t("pages:archivedChats.deleteFailed")));
    } finally {
      setBusy(false);
      setDeleteCodes([]);
    }
  };

  const downloadAttachment = async (session: ArchivedSession, attachment: NonNullable<ArchivedMessage["attachments"]>[number]) => {
    try {
      const response = await downloadArchivedChatAttachment(session.sessionCode, attachment.resourceCode || attachment.id);
      const url = URL.createObjectURL(response.blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = attachment.fileName || attachment.title || attachment.summary || attachment.id;
      link.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (error) {
      toast.error(getErrorMessage(error, t("pages:archivedChats.attachmentFailed")));
    }
  };

  const nativeLabel = (group: string, value: string) => t(`pages:archivedChats.${group}.${value}`, { defaultValue: value });

  const allSelected = sessions.length > 0 && selectedCodes.size === sessions.length;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));

  return (
    <>
      <PageHeader title={t("pages:archivedChats.title")} />
      <DataTable<ArchivedSession>
        rows={loading ? [] : sessions}
        rowKey={(session) => session.sessionCode}
        tableClassName="table-fixed min-w-[800px]"
        isRowSelected={(session) => selectedCodes.has(session.sessionCode)}
        emptyState={t(loading ? "pages:archivedChats.loading" : "pages:archivedChats.empty")}
        toolbar={(
          <>
            <SelectInput
              aria-label={t("pages:archivedChats.workspace")}
              className="w-full sm:w-64"
              value={workspaceCode || "__all__"}
              onValueChange={(value) => setWorkspaceCode(value === "__all__" ? "" : String(value))}
            >
              <SelectItem value="__all__">{t("pages:archivedChats.allWorkspaces")}</SelectItem>
              {workspaceOptions.map((workspace) => <SelectItem key={workspace.workspaceCode} value={workspace.workspaceCode}>{workspace.label || workspace.name}</SelectItem>)}
            </SelectInput>
            <div className="ml-auto flex items-center gap-2">
              {failedCodes.length > 0 && <Button variant="secondary" disabled={busy} onClick={() => void deleteSelected(failedCodes)}>{t("pages:archivedChats.retryFailures", { count: failedCodes.length })}</Button>}
              {selectedCodes.size > 0 && <Button variant="destructive" disabled={busy} onClick={() => void deleteSelected([...selectedCodes])}>{t("pages:archivedChats.deleteSelected", { count: selectedCodes.size })}</Button>}
              <Button variant="ghost" size="sm" aria-label={t("pages:archivedChats.refresh")} title={t("pages:archivedChats.refresh")} disabled={loading || busy} onClick={() => void loadSessions(current)}>{t("pages:archivedChats.refresh")}</Button>
            </div>
          </>
        )}
        pagination={<TablePagination
          itemLabel={t("pages:archivedChats.itemLabel")}
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
          { id: "selection", width: "56px", header: <input type="checkbox" aria-label={t("pages:archivedChats.selectPage")} checked={allSelected} disabled={loading || busy || sessions.length === 0} onChange={toggleAll} />, render: (session) => <input type="checkbox" aria-label={t("pages:archivedChats.selectSession", { title: session.title })} checked={selectedCodes.has(session.sessionCode)} disabled={busy} onChange={() => toggleSelected(session.sessionCode)} /> },
          { id: "session", header: t("pages:archivedChats.session"), cellClassName: "min-w-0", render: (session) => <p className="truncate font-medium" title={session.title}>{session.title}</p> },
          { id: "workspace", header: t("pages:archivedChats.workspace"), width: "180px", render: (session) => <p className="truncate" title={session.workspaceCode}>{workspaceLabels.get(session.workspaceCode) ?? session.workspaceCode}</p> },
          { id: "updatedAt", header: t("pages:archivedChats.updatedAt"), width: "210px", render: (session) => <time className="text-xs">{displayTimestamp(session.updatedAt)}</time> },
          { id: "actions", header: t("pages:archivedChats.actions"), width: "152px", align: "right", headerClassName: "sticky right-0 z-20 border-l border-[var(--color-outline-variant)] bg-[var(--tron-table-header-bg,var(--color-elevated-surface))]", cellClassName: "sticky right-0 z-10 border-l border-[var(--color-outline-variant)] bg-[var(--_tron-row-bg)]", render: (session) => <div className="flex justify-end gap-1">
            <Button variant="ghost" size="sm" title={t("pages:archivedChats.view")} aria-label={t("pages:archivedChats.view")} disabled={busy} onClick={() => void openSession(session)}>{t("pages:archivedChats.view")}</Button>
            <Button variant="ghost" size="sm" title={t("pages:archivedChats.restore")} aria-label={t("pages:archivedChats.restore")} disabled={busy} onClick={() => void restoreSession(session)}>{t("pages:archivedChats.restore")}</Button>
            <Button variant="ghost" size="sm" className="text-[var(--color-error)]" title={t("pages:archivedChats.delete")} aria-label={t("pages:archivedChats.delete")} disabled={busy} onClick={() => void deleteSelected([session.sessionCode])}>{t("pages:archivedChats.delete")}</Button>
          </div> },
        ]}
      />
        {deleteErrors.length > 0 && (
          <ul className="space-y-1 rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error-container)]/30 p-3 text-xs" aria-live="polite">
            {deleteErrors.map((item) => (
              <li key={item.sessionCode}>
                <span className="font-mono">{item.sessionCode}</span>: {t(`pages:archivedChats.errors.${item.errorCode}`, { defaultValue: t("pages:archivedChats.deleteFailed") })}
              </li>
            ))}
          </ul>
        )}

      {viewing && (
        <Dialog open onOpenChange={(open) => { if (!open) { ++detailRequest.current; setViewing(null); } }}>
          <DialogContent className="max-w-3xl" closeLabel={t("common:actions.close")}>
            <DialogHeader>
              <DialogTitle className="truncate">{viewing.title}</DialogTitle>
            </DialogHeader>
            <div className="max-h-[70vh] overflow-y-auto">

              {detail && (detail.goals.length > 0 || detail.runs.length > 0) && (
                <div className="mb-4 space-y-2 text-sm">
                  {detail.goals.map((goal) => <details key={goal.goalCode} className="rounded-lg border border-[var(--color-outline-variant)] p-3">
                    <summary className="cursor-pointer">{goal.objective} · {nativeLabel("statuses", goal.status)} · {goal.runsUsed}/{goal.maxRuns}</summary>
                    {goal.reason && <p className="mt-2">{goal.reason}</p>}
                    {goal.criteria.length > 0 && <ul className="mt-2 space-y-1">{goal.criteria.map((criterion) => <li key={criterion.id}>{criterion.completed ? "✓" : "○"} {criterion.text}</li>)}</ul>}
                    {goal.lastDecision && <p className="mt-2">{nativeLabel("verdicts", goal.lastDecision.verdict)} · {goal.lastDecision.reason}{goal.lastDecision.required_input ? ` · ${goal.lastDecision.required_input}` : ""}</p>}
                    {goal.progress != null && <pre className="mt-2 whitespace-pre-wrap break-words text-xs">{JSON.stringify(goal.progress, null, 2)}</pre>}
                    <p className="mt-3 font-medium">{t("pages:archivedChats.goalRecords")}</p>
                    <ol className="mt-3 space-y-2">{detail.goalEvents.filter((event) => event.goalCode === goal.goalCode).map((event) => <li key={event.eventCode} className="border-t border-[var(--color-outline-variant)] pt-2 text-xs">
                      <p>{displayTimestamp(event.createdAt)} · {nativeLabel("eventKinds", event.kind)}{event.runCode ? ` · ${event.runCode}` : ""}</p>
                      <pre className="whitespace-pre-wrap break-words">{JSON.stringify(event.payload, null, 2)}</pre>
                    </li>)}</ol>
                  </details>)}
                  {detail.runs.map((run) => <p key={run.runCode} className="font-mono text-xs">{nativeLabel("runKinds", run.kind)} · {nativeLabel("statuses", run.status)} · {run.runCode}</p>)}
                </div>
              )}
              {archivedA2ui?.messages().map(message => <ChatA2UI key={message.messageId} message={message} runtime={archivedA2ui} disabled />)}
              {detail?.history.map((branch) => <details key={`${branch.branch}:${branch.generation}`} className="mb-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
                <summary className="cursor-pointer text-xs">{branch.branch} · {branch.messages.length}</summary>
                <ol className="mt-2 space-y-2">{branch.messages.map((message) => <li key={message.messageCode} className="whitespace-pre-wrap break-words text-xs"><span className="font-medium">{nativeLabel("roles", message.role)}</span>: {message.content}</li>)}</ol>
              </details>)}
              {messagesLoading ? <p>{t("pages:archivedChats.loading")}</p> : messages.length === 0 ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:archivedChats.noMessages")}</p> : (
                <ol className="space-y-4">
                  {messages.map((message) => (
                    <li key={message.messageCode} className="rounded-lg border border-[var(--color-outline-variant)] p-3">
                      <div className="mb-2 flex justify-between gap-3 text-xs text-[var(--color-on-surface-variant)]"><span>{nativeLabel("roles", message.role)} · {nativeLabel("roles", message.actor)}</span><time>{displayTimestamp(message.createdAt)}</time></div>
                      <p className="whitespace-pre-wrap break-words text-sm">{message.content}</p>
                      {(message.attachments?.length ?? 0) > 0 && <ul className="mt-2 space-y-1 border-t border-[var(--color-outline-variant)] pt-2 text-xs">{message.attachments?.map((attachment) => <li key={attachment.id}><button type="button" className="text-left underline underline-offset-2" onClick={() => void downloadAttachment(viewing, attachment)}>📎 {attachment.fileName || attachment.title || attachment.summary}</button></li>)}</ul>}
                    </li>
                  ))}
                </ol>
              )}
            </div>
          </DialogContent>
        </Dialog>
      )}
      <ConfirmDialog
        open={deleteCodes.length > 0}
        title={t("pages:archivedChats.deleteTitle")}
        description={t("pages:archivedChats.confirmDelete", { count: deleteCodes.length })}
        cancelText={t("pages:archivedChats.cancel")}
        confirmText={t("pages:archivedChats.delete")}
        pending={busy}
        onCancel={() => setDeleteCodes([])}
        onConfirm={() => void confirmDelete()}
      />
    </>
  );
}
