import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Eye, RefreshCcw, RotateCcw, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { PageHeader, SurfaceCard } from "@/shared";
import { fetchWorkspaceOptions } from "@/domain/admin/api";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";
import type { ChatMessage, ChatSession } from "@/domain/chat/model";
import {
  deleteArchivedChatSessions,
  fetchArchivedChatMessages,
  fetchArchivedChatSessions,
  restoreArchivedChatSession,
  type ArchivedChatSessionDeleteItem,
} from "@/domain/admin/api/archivedChatsApi";
import { useToast } from "@/components/ui/toast";
import { downloadChatSessionAttachment } from "@/domain/chat/api/chatAttachmentApi";

const PAGE_SIZE = 25;

function getErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function displayTimestamp(value: string) {
  return value.replace("T", " ").replace(/\.\d+(Z|[+-]\d{2}:?\d{2})?$/, "");
}

export function ArchivedChatsPage() {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const [workspaceOptions, setWorkspaceOptions] = useState<WorkspaceOption[]>([]);
  const [workspaceCode, setWorkspaceCode] = useState("");
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState(1);
  const [selectedCodes, setSelectedCodes] = useState<Set<string>>(new Set());
  const [failedCodes, setFailedCodes] = useState<string[]>([]);
  const [deleteErrors, setDeleteErrors] = useState<ArchivedChatSessionDeleteItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [viewing, setViewing] = useState<ChatSession | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);

  const workspaceLabels = useMemo(
    () => new Map(workspaceOptions.map((workspace) => [workspace.workspaceCode, workspace.label || workspace.name])),
    [workspaceOptions],
  );

  const loadSessions = useCallback(async (page: number) => {
    setLoading(true);
    try {
      const query = {
        workspaceCode: workspaceCode || undefined,
        current: page,
        size: PAGE_SIZE,
      };
      let result = await fetchArchivedChatSessions(query);
      const lastPage = Math.max(1, Math.ceil(result.total / PAGE_SIZE));
      if (page > lastPage) {
        result = await fetchArchivedChatSessions({ ...query, current: lastPage });
      }
      setSessions(result.records);
      setTotal(result.total);
      setCurrent(result.current);
      setSelectedCodes(new Set());
    } catch (error) {
      toast.error(getErrorMessage(error, t("pages:archivedChats.loadFailed")));
    } finally {
      setLoading(false);
    }
  }, [toast, t, workspaceCode]);

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
    void loadSessions(1);
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

  const openSession = async (session: ChatSession) => {
    setViewing(session);
    setMessages([]);
    setMessagesLoading(true);
    try {
      setMessages(await fetchArchivedChatMessages(session.sessionCode));
    } catch (error) {
      toast.error(getErrorMessage(error, t("pages:archivedChats.viewFailed")));
    } finally {
      setMessagesLoading(false);
    }
  };

  const restoreSession = async (session: ChatSession) => {
    setBusy(true);
    try {
      await restoreArchivedChatSession(session.sessionCode);
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

  const deleteSelected = async (codes: string[]) => {
    if (codes.length === 0) return;
    const confirmed = window.confirm(t("pages:archivedChats.confirmDelete", { count: codes.length }));
    if (!confirmed) return;
    setBusy(true);
    try {
      const result = await deleteArchivedChatSessions(codes);
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
    }
  };

  const downloadAttachment = async (session: ChatSession, attachment: NonNullable<ChatMessage["attachments"]>[number]) => {
    try {
      const response = await downloadChatSessionAttachment(session, attachment);
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

  const allSelected = sessions.length > 0 && selectedCodes.size === sessions.length;
  const pageCount = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <>
      <PageHeader
        title={t("pages:archivedChats.title")}
        actions={
          <Button variant="secondary" disabled={loading || busy} onClick={() => void loadSessions(current)}>
            <RefreshCcw />
            {t("pages:archivedChats.refresh")}
          </Button>
        }
      />
      <SurfaceCard className="space-y-4">
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-sm">
            <span>{t("pages:archivedChats.workspace")}</span>
            <select
              className="h-9 min-w-56 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface)] px-3"
              value={workspaceCode}
              onChange={(event) => { setCurrent(1); setWorkspaceCode(event.target.value); }}
            >
              <option value="">{t("pages:archivedChats.allWorkspaces")}</option>
              {workspaceOptions.map((workspace) => (
                <option key={workspace.workspaceCode} value={workspace.workspaceCode}>
                  {workspace.label || workspace.name} · {workspace.workspaceCode}
                </option>
              ))}
            </select>
          </label>
          <span className="text-sm text-[var(--color-on-surface-variant)]">
            {t("pages:archivedChats.total", { count: total })}
          </span>
          <div className="ml-auto flex flex-wrap gap-2">
            {failedCodes.length > 0 && (
              <Button variant="secondary" disabled={busy} onClick={() => void deleteSelected(failedCodes)}>
                <RefreshCcw />
                {t("pages:archivedChats.retryFailures", { count: failedCodes.length })}
              </Button>
            )}
            <Button variant="destructive" disabled={busy || selectedCodes.size === 0} onClick={() => void deleteSelected([...selectedCodes])}>
              <Trash2 />
              {t("pages:archivedChats.deleteSelected", { count: selectedCodes.size })}
            </Button>
          </div>
        </div>

        <div className="overflow-x-auto rounded-lg border border-[var(--color-outline-variant)]">
          <table className="w-full min-w-[900px] text-left text-sm">
            <thead className="bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
              <tr>
                <th className="w-10 px-3 py-3"><input type="checkbox" aria-label={t("pages:archivedChats.selectPage")} checked={allSelected} onChange={toggleAll} /></th>
                <th className="px-3 py-3">{t("pages:archivedChats.session")}</th>
                <th className="px-3 py-3">{t("pages:archivedChats.workspace")}</th>
                <th className="px-3 py-3">{t("pages:archivedChats.updatedAt")}</th>
                <th className="px-3 py-3 text-right">{t("pages:archivedChats.actions")}</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                <tr><td colSpan={5} className="px-3 py-10 text-center text-[var(--color-on-surface-variant)]">{t("pages:archivedChats.loading")}</td></tr>
              ) : sessions.length === 0 ? (
                <tr><td colSpan={5} className="px-3 py-10 text-center text-[var(--color-on-surface-variant)]">{t("pages:archivedChats.empty")}</td></tr>
              ) : sessions.map((session) => (
                <tr key={session.sessionCode} className="border-t border-[var(--color-outline-variant)]">
                  <td className="px-3 py-3"><input type="checkbox" aria-label={t("pages:archivedChats.selectSession", { title: session.title })} checked={selectedCodes.has(session.sessionCode)} onChange={() => toggleSelected(session.sessionCode)} /></td>
                  <td className="px-3 py-3">
                    <div className="font-medium">{session.title}</div>
                    <div className="font-mono text-xs text-[var(--color-on-surface-variant)]">{session.sessionCode}</div>
                  </td>
                  <td className="px-3 py-3">
                    <div>{workspaceLabels.get(session.workspaceCode) ?? session.workspaceCode}</div>
                    <div className="font-mono text-xs text-[var(--color-on-surface-variant)]">{session.workspaceCode}</div>
                  </td>
                  <td className="px-3 py-3 font-mono text-xs">{displayTimestamp(session.updatedAt)}</td>
                  <td className="px-3 py-3">
                    <div className="flex justify-end gap-2">
                      <Button variant="secondary" size="sm" disabled={busy} onClick={() => void openSession(session)}><Eye />{t("pages:archivedChats.view")}</Button>
                      <Button variant="secondary" size="sm" disabled={busy} onClick={() => void restoreSession(session)}><RotateCcw />{t("pages:archivedChats.restore")}</Button>
                      <Button variant="destructive" size="sm" disabled={busy} onClick={() => void deleteSelected([session.sessionCode])}><Trash2 />{t("pages:archivedChats.delete")}</Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="flex items-center justify-between text-sm text-[var(--color-on-surface-variant)]">
          <span>{t("pages:archivedChats.pageOf", { current, total: pageCount })}</span>
          <div className="flex gap-2">
            <Button variant="secondary" size="sm" disabled={loading || current <= 1} onClick={() => void loadSessions(current - 1)}>{t("pages:archivedChats.previous")}</Button>
            <Button variant="secondary" size="sm" disabled={loading || current >= pageCount} onClick={() => void loadSessions(current + 1)}>{t("pages:archivedChats.next")}</Button>
          </div>
        </div>
        {deleteErrors.length > 0 && (
          <ul className="space-y-1 rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error-container)]/30 p-3 text-xs" aria-live="polite">
            {deleteErrors.map((item) => (
              <li key={item.sessionCode}>
                <span className="font-mono">{item.sessionCode}</span>: {item.message || t("pages:archivedChats.deleteFailed")}
              </li>
            ))}
          </ul>
        )}
      </SurfaceCard>

      {viewing && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setViewing(null); }}>
          <section className="flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded-xl bg-[var(--color-surface)] shadow-xl" role="dialog" aria-modal="true" aria-labelledby="archived-chat-title">
            <header className="flex items-start justify-between gap-4 border-b border-[var(--color-outline-variant)] p-4">
              <div className="min-w-0"><h2 id="archived-chat-title" className="truncate font-semibold">{viewing.title}</h2><p className="font-mono text-xs text-[var(--color-on-surface-variant)]">{viewing.sessionCode}</p></div>
              <Button variant="secondary" size="sm" onClick={() => setViewing(null)}>{t("pages:archivedChats.close")}</Button>
            </header>
            <div className="overflow-y-auto p-4">
              {messagesLoading ? <p>{t("pages:archivedChats.loading")}</p> : messages.length === 0 ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:archivedChats.noMessages")}</p> : (
                <ol className="space-y-4">
                  {messages.map((message) => (
                    <li key={message.messageCode} className="rounded-lg border border-[var(--color-outline-variant)] p-3">
                      <div className="mb-2 flex justify-between gap-3 text-xs text-[var(--color-on-surface-variant)]"><span>{message.role} · {message.actor}</span><time>{displayTimestamp(message.createdAt)}</time></div>
                      <p className="whitespace-pre-wrap break-words text-sm">{message.content}</p>
                      {(message.attachments?.length ?? 0) > 0 && <ul className="mt-2 space-y-1 border-t border-[var(--color-outline-variant)] pt-2 text-xs">{message.attachments?.map((attachment) => <li key={attachment.id}><button type="button" className="text-left underline underline-offset-2" onClick={() => void downloadAttachment(viewing, attachment)}>📎 {attachment.fileName || attachment.title || attachment.summary}</button></li>)}</ul>}
                    </li>
                  ))}
                </ol>
              )}
            </div>
          </section>
        </div>
      )}
    </>
  );
}
