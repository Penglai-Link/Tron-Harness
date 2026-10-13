import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";
import { Archive, ExternalLink, LoaderCircle, MailCheck, MailOpen, RotateCw, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useTabVisibility } from "@/components/editable-tabs";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import {
  listNotifications,
  notificationQueryKey,
  notificationTarget,
  updateNotification,
  deleteNotification,
  deleteArchivedNotifications,
  type PlatformNotification,
} from "../api/notificationApi";
export function NotificationsPage() {
  const { t, i18n } = useTranslation("pages");
  const client = useQueryClient();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const active = useTabVisibility();
  const [filter, setFilter] = useState<"all" | "unread" | "read" | "archived">("unread");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PlatformNotification | "all" | null>(null);
  const [deletePending, setDeletePending] = useState(false);
  const [detailReadId, setDetailReadId] = useState<string | null>(null);
  const query = useQuery({
    queryKey: notificationQueryKey,
    queryFn: ({ signal }) => listNotifications(signal),
    enabled: active,
    refetchInterval: active ? 15000 : false,
  });
  async function change(id: string, read: boolean, archived: boolean, pendingKey = id) {
    setBusy(pendingKey);
    setError(null);
    try {
      await updateNotification(id, { read, archived });
      await client.invalidateQueries({ queryKey: notificationQueryKey });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(null);
    }
  }
  async function changeAll(action: "read" | "archive") {
    const items = query.data ?? [];
    const pending = action === "read"
      ? items.filter((item) => item.archivedAt === null && item.readAt === null)
      : items.filter((item) => item.archivedAt === null);
    if (pending.length === 0) return;
    setBusy(action);
    setError(null);
    try {
      await Promise.all(pending.map((item) => updateNotification(item.id, {
        read: true,
        archived: action === "archive" ? true : item.archivedAt !== null,
      })));
      await client.invalidateQueries({ queryKey: notificationQueryKey });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      await client.invalidateQueries({ queryKey: notificationQueryKey });
    } finally {
      setBusy(null);
    }
  }
  async function removeArchived() {
    if (!deleteTarget) return;
    setDeletePending(true);
    setError(null);
    try {
      if (deleteTarget === "all") await deleteArchivedNotifications();
      else await deleteNotification(deleteTarget.id);
      setDeleteTarget(null);
      await client.invalidateQueries({ queryKey: notificationQueryKey });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setDeletePending(false);
    }
  }
  const records = (query.data ?? []).filter((item) =>
    filter === "archived"
      ? item.archivedAt !== null
      : item.archivedAt === null && (filter === "all" || (filter === "unread" ? item.readAt === null : item.readAt !== null))
  );
  const selectedNotificationId = searchParams.get("notification");
  const selectedNotification = selectedNotificationId
    ? (query.data ?? []).find((item) => item.id === selectedNotificationId) ?? null
    : null;
  useEffect(() => {
    if (!active || !selectedNotification || selectedNotification.archivedAt !== null || selectedNotification.readAt !== null || detailReadId === selectedNotification.id) return;
    setDetailReadId(selectedNotification.id);
    void change(selectedNotification.id, true, false, `read:${selectedNotification.id}`);
  }, [active, detailReadId, selectedNotification]);
  function closeDetails() {
    const next = new URLSearchParams(searchParams);
    next.delete("notification");
    setSearchParams(next, { replace: true });
  }
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <Tabs
          value={filter}
          onValueChange={(value) => {
            if (value === "all" || value === "unread" || value === "read" || value === "archived") setFilter(value);
          }}
        >
          <TabsList variant="line" aria-label={t("notifications.filters")}>
            {(["all", "unread", "read", "archived"] as const).map((value) => (
              <TabsTrigger className="flex-none px-3" key={value} value={value}>
                {t(`notifications.${value}`)}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
        <div className="flex items-center gap-1">
          {filter === "archived" ? (
            <Button size="sm" variant="destructive" disabled={deletePending || !(query.data ?? []).some((item) => item.archivedAt !== null)} onClick={() => setDeleteTarget("all")}>
              {t("notifications.deleteAll")}
            </Button>
          ) : (
            <>
              {filter !== "read" ? (
                <Button size="sm" variant="outline" aria-busy={busy === "read"} disabled={busy !== null || deletePending || !(query.data ?? []).some((item) => item.archivedAt === null && item.readAt === null)} onClick={() => void changeAll("read")}>
                  {busy === "read" ? <LoaderCircle className="animate-spin" size={14} aria-hidden="true" /> : null}
                  {t(busy === "read" ? "notifications.markAllReadPending" : "notifications.markAllRead")}
                </Button>
              ) : null}
              <Button size="sm" variant="outline" aria-busy={busy === "archive"} disabled={busy !== null || deletePending || !(query.data ?? []).some((item) => item.archivedAt === null)} onClick={() => void changeAll("archive")}>
                {busy === "archive" ? <LoaderCircle className="animate-spin" size={14} aria-hidden="true" /> : null}
                {t(busy === "archive" ? "notifications.archiveAllPending" : "notifications.archiveAll")}
              </Button>
            </>
          )}
          <Button size="sm" variant="outline" onClick={() => void query.refetch()}>
            {t("dude.refresh")}
          </Button>
        </div>
      </div>
      {query.isPending ? <p role="status">{t("dude.loading")}</p> : null}
      {query.error || error ? (
        <div
          role="alert"
          className="flex items-center gap-3 text-[var(--color-error)]"
        >
          {error ?? query.error?.message}
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              setError(null);
              void query.refetch();
            }}
          >
            {t("dude.retry")}
          </Button>
        </div>
      ) : null}
      <div className="divide-y divide-[var(--color-outline-variant)]">
        {records.map((item) => {
          const changeKind = typeof item.target.changeKind === "string" ? item.target.changeKind : "";
          const requestedAction = typeof item.target.requestedAction === "string" ? item.target.requestedAction : "";
          const resolution = typeof item.target.resolution === "string" ? item.target.resolution : "";
          const evidence = Array.isArray(item.target.evidence) ? item.target.evidence.filter((value): value is string => typeof value === "string") : [];
          return (
            <article
              key={item.id}
              className="flex cursor-pointer flex-wrap items-start gap-3 py-4"
              onClick={() => {
                const next = new URLSearchParams(searchParams);
                next.set("notification", item.id);
                setSearchParams(next);
              }}
              onKeyDown={(event) => {
                if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) {
                  event.preventDefault();
                  const next = new URLSearchParams(searchParams);
                  next.set("notification", item.id);
                  setSearchParams(next);
                }
              }}
              role="button"
              tabIndex={0}
              aria-label={item.title}
            >
              <div className="min-w-0 basis-full sm:flex-1 sm:basis-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="break-words font-medium">{item.title}</h2>
                  {["Progress", "Completed", "NeedsDecision", "Blocked"].includes(changeKind) ? <Badge variant="outline">{t(`notifications.changes.${changeKind}`)}</Badge> : null}
                  {item.readAt === null ? (
                    <Badge variant="secondary">
                      {t("notifications.unread")}
                    </Badge>
                  ) : null}
                  {item.resolvedAt !== null ? (
                    <Badge variant="outline">
                      {t("notifications.resolved")}
                    </Badge>
                  ) : null}
                </div>
                <p className="mt-1 whitespace-pre-wrap break-words text-sm">
                  {item.body}
                </p>
                {requestedAction && item.resolvedAt === null ? <p className="mt-2 whitespace-pre-wrap break-words text-sm"><span className="font-medium">{t("notifications.requestedAction")}：</span>{requestedAction}</p> : null}
                {resolution ? <p className="mt-2 whitespace-pre-wrap break-words text-sm text-[var(--color-on-surface-variant)]"><span className="font-medium">{t("notifications.resolution")}：</span>{resolution}</p> : null}
                {evidence.length ? <details className="mt-2 text-xs text-[var(--color-on-surface-variant)]" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}><summary className="cursor-pointer">{t("notifications.evidence")}</summary><ul className="mt-1 list-inside list-disc space-y-1">{evidence.map((entry) => <li key={entry} className="break-all">{entry}</li>)}</ul></details> : null}
                <p className="mt-2 text-xs text-[var(--color-on-surface-variant)]">
                  {item.dudeCode ?? t("notifications.platform")} ·{" "}
                  {new Intl.DateTimeFormat(i18n.language, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }).format(item.createdAt)}
                </p>
              </div>
              <div className="flex flex-wrap gap-1">
                {item.archivedAt !== null ? (
                  <Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label={t("notifications.delete")}
                    title={t("notifications.delete")}
                    disabled={deletePending}
                    onClick={(event) => {
                      event.stopPropagation();
                      setDeleteTarget(item);
                    }}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                ) : (
                  <>
                    <Button
                  size="icon-sm"
                  variant="ghost"
                  aria-label={t(item.readAt === null ? "notifications.markRead" : "notifications.markUnread")}
                  title={t(item.readAt === null ? "notifications.markRead" : "notifications.markUnread")}
                    aria-busy={busy === `read:${item.id}`}
                  disabled={busy !== null || deletePending}
                  onClick={(event) => {
                    event.stopPropagation();
                    void change(
                      item.id,
                      item.readAt === null,
                      item.archivedAt !== null,
                      `read:${item.id}`
                    );
                  }}
                >
                  {busy === `read:${item.id}` ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : item.readAt === null ? <MailCheck aria-hidden="true" /> : <MailOpen aria-hidden="true" />}
                </Button>
                <Button
                  size="icon-sm"
                  variant="ghost"
                  aria-label={t(item.archivedAt === null ? "notifications.archive" : "notifications.restore")}
                  title={t(item.archivedAt === null ? "notifications.archive" : "notifications.restore")}
                  aria-busy={busy === `archive:${item.id}`}
                  disabled={busy !== null || deletePending}
                  onClick={(event) => {
                    event.stopPropagation();
                    void change(
                      item.id,
                      item.readAt !== null,
                      item.archivedAt === null,
                      `archive:${item.id}`
                    );
                  }}
                >
                  {busy === `archive:${item.id}` ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : item.archivedAt === null ? <Archive aria-hidden="true" /> : <RotateCw aria-hidden="true" />}
                </Button>
                  </>
                )}
              </div>
            </article>
          );
        })}
      </div>
      {!query.isPending && !query.error && records.length === 0 ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {t("notifications.empty")}
        </p>
      ) : null}
      <Dialog open={Boolean(selectedNotificationId)} onOpenChange={(open) => { if (!open) closeDetails(); }}>
        {selectedNotification ? <DialogContent className="max-h-[85dvh] overflow-y-auto sm:max-w-xl" closeLabel={t("dude.close")}>
          <DialogHeader>
            <DialogTitle>{selectedNotification.title}</DialogTitle>
            <DialogDescription>{selectedNotification.dudeCode ?? t("notifications.platform")} · {new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(selectedNotification.createdAt)}</DialogDescription>
          </DialogHeader>
          <div className="space-y-3 text-sm">
            <p className="whitespace-pre-wrap break-words">{selectedNotification.body}</p>
            {typeof selectedNotification.target.changeKind === "string" ? <p><Badge variant="outline">{t(`notifications.changes.${selectedNotification.target.changeKind}`)}</Badge></p> : null}
            {typeof selectedNotification.target.requestedAction === "string" ? <p className="whitespace-pre-wrap break-words"><span className="font-medium">{t("notifications.requestedAction")}：</span>{selectedNotification.target.requestedAction}</p> : null}
            {typeof selectedNotification.target.resolution === "string" ? <p className="whitespace-pre-wrap break-words"><span className="font-medium">{t("notifications.resolution")}：</span>{selectedNotification.target.resolution}</p> : null}
            {Array.isArray(selectedNotification.target.evidence) && selectedNotification.target.evidence.some((value) => typeof value === "string") ? <div><p className="font-medium">{t("notifications.evidence")}</p><ul className="mt-1 list-inside list-disc space-y-1 text-[var(--color-on-surface-variant)]">{selectedNotification.target.evidence.filter((value): value is string => typeof value === "string").map((entry) => <li key={entry} className="break-all">{entry}</li>)}</ul></div> : null}
          </div>
          <DialogFooter>
            {notificationTarget(selectedNotification) ? <Button variant="outline" onClick={() => navigate(notificationTarget(selectedNotification)!)}><ExternalLink aria-hidden="true" />{t("notifications.openRelated")}</Button> : null}
            <Button onClick={closeDetails}>{t("dude.close")}</Button>
          </DialogFooter>
        </DialogContent> : null}
      </Dialog>
      <ConfirmDialog
        open={deleteTarget !== null}
        title={t(deleteTarget === "all" ? "notifications.deleteAllTitle" : "notifications.deleteTitle")}
        description={t(deleteTarget === "all" ? "notifications.deleteAllDescription" : "notifications.deleteDescription", { count: query.data?.filter((item) => item.archivedAt !== null).length ?? 0 })}
        cancelText={t("dude.cancel")}
        confirmText={t(deleteTarget === "all" ? "notifications.deleteAll" : "notifications.delete")}
        pendingText={t("notifications.deletePending")}
        pending={deletePending}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => void removeArchived()}
      />
    </div>
  );
}
