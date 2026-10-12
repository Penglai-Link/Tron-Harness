import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Archive, LoaderCircle, MailCheck, MailOpen, RotateCw, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useTabVisibility } from "@/components/editable-tabs";
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
  const active = useTabVisibility();
  const [filter, setFilter] = useState<"all" | "unread" | "read" | "archived">("unread");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PlatformNotification | "all" | null>(null);
  const [deletePending, setDeletePending] = useState(false);
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
  async function openNotification(item: PlatformNotification, target: string) {
    if (item.archivedAt === null && item.readAt === null) await change(item.id, true, false, `read:${item.id}`);
    navigate(target);
  }
  const records = (query.data ?? []).filter((item) =>
    filter === "archived"
      ? item.archivedAt !== null
      : item.archivedAt === null && (filter === "all" || (filter === "unread" ? item.readAt === null : item.readAt !== null))
  );
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
          const target = notificationTarget(item);
          return (
            <article
              key={item.id}
              className={`flex flex-wrap items-start gap-3 py-4 ${target ? "cursor-pointer" : ""}`}
              onClick={target ? () => void openNotification(item, target) : undefined}
              onKeyDown={target ? (event) => { if (event.target === event.currentTarget && event.key === "Enter") { event.preventDefault(); void openNotification(item, target); } } : undefined}
              role={target ? "link" : undefined}
              tabIndex={target ? 0 : undefined}
              aria-label={target ? item.title : undefined}
            >
              <div className="min-w-0 basis-full sm:flex-1 sm:basis-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="break-words font-medium">{item.title}</h2>
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
