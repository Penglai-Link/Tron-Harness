import { useCallback, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useSearchParams } from "react-router";
import { useTabStore } from "@/store/tabStore";
import { Inbox, LoaderCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useTabVisibility } from "@/components/editable-tabs";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { NotificationListItem } from "../components/NotificationListItem";
import { NotificationDetailsDialog } from "../components/NotificationDetailsDialog";
import {
  listNotifications,
  notificationQueryKey,
  updateNotification,
  deleteNotification,
  deleteArchivedNotifications,
  type PlatformNotification,
} from "../api/notificationApi";
export function NotificationsPage() {
  const { t } = useTranslation("pages");
  const client = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const active = useTabVisibility();
  const toolbar = useRef<HTMLElement>(null);
  const detailTrigger = useRef<HTMLButtonElement | null>(null);
  const [filter, setFilter] = useState<"all" | "unread" | "read" | "archived">("unread");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PlatformNotification | "all" | null>(null);
  const [deletePending, setDeletePending] = useState(false);
  const [dismissedId, setDismissedId] = useState<string | null>(null);
  const [detailReadId, setDetailReadId] = useState<string | null>(null);
  const query = useQuery({
    queryKey: notificationQueryKey,
    queryFn: ({ signal }) => listNotifications(signal),
    enabled: active,
    refetchInterval: active ? 15000 : false,
  });
  const change = useCallback(async (id: string, read: boolean, archived: boolean, pendingKey = id) => {
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
  }, [client]);
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
  const notifications = query.data ?? [];
  const counts = {
    all: notifications.filter((item) => item.archivedAt === null).length,
    unread: notifications.filter((item) => item.archivedAt === null && item.readAt === null).length,
    read: notifications.filter((item) => item.archivedAt === null && item.readAt !== null).length,
    archived: notifications.filter((item) => item.archivedAt !== null).length,
  };
  const records = notifications.filter((item) =>
    filter === "archived"
      ? item.archivedAt !== null
      : item.archivedAt === null && (filter === "all" || (filter === "unread" ? item.readAt === null : item.readAt !== null))
  ).sort((left, right) => right.createdAt - left.createdAt);
  const selectedNotificationId = searchParams.get("notification");
  const selectedNotification = selectedNotificationId
    ? (query.data ?? []).find((item) => item.id === selectedNotificationId) ?? null
    : null;
  const detailsOpen = active && Boolean(selectedNotificationId) && selectedNotificationId !== dismissedId;
  useEffect(() => {
    if (!detailsOpen || !selectedNotification || selectedNotification.archivedAt !== null || selectedNotification.readAt !== null || detailReadId === selectedNotification.id) return;
    setDetailReadId(selectedNotification.id);
    void change(selectedNotification.id, true, false, `read:${selectedNotification.id}`);
  }, [change, detailReadId, detailsOpen, selectedNotification]);
  function closeDetails() {
    setDismissedId(selectedNotificationId);
    const next = new URLSearchParams(searchParams);
    next.delete("notification");
    const tabs = useTabStore.getState();
    const current = tabs.tabs.find((tab) => tab.id === tabs.activeTabId);
    if (active && current && current.path.split("?")[0] === location.pathname) {
      const query = next.toString();
      tabs.updateTab(current.id, { path: `${location.pathname}${query ? `?${query}` : ""}` });
    }
    setSearchParams(next, { replace: true });
  }
  return (
    <div className="min-w-0 pb-6">
      <SurfaceCard className="overflow-hidden p-0 shadow-none">
        <header ref={toolbar} className="flex flex-wrap items-center justify-between gap-x-4 gap-y-3 border-b border-[var(--color-outline-variant)] px-4 py-3 sm:px-5">
          <Tabs className="min-w-0 max-w-full overflow-x-auto" value={filter} onValueChange={(value) => {
            if (value === "all" || value === "unread" || value === "read" || value === "archived") setFilter(value);
          }}>
            <TabsList variant="line" aria-label={t("notifications.filters")}>
              {(["all", "unread", "read", "archived"] as const).map((value) => (
                <TabsTrigger className="flex-none gap-2 px-2.5" key={value} value={value}>
                  {t(`notifications.${value}`)}
                  {query.data ? <span className="text-xs font-normal text-[var(--color-on-surface-variant)] tabular-nums">{counts[value]}</span> : null}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
          <div className="ml-auto flex flex-wrap items-center gap-1.5">
            {filter === "archived" ? (
              <Button size="sm" variant="destructive" disabled={busy !== null || deletePending || counts.archived === 0} onClick={() => setDeleteTarget("all")}>
                {t("notifications.deleteAll")}
              </Button>
            ) : <>
              {filter !== "read" ? <Button size="sm" variant="outline" aria-busy={busy === "read"} disabled={busy !== null || deletePending || counts.unread === 0} onClick={() => void changeAll("read")}>
                {busy === "read" ? <LoaderCircle className="animate-spin" size={14} aria-hidden="true" /> : null}
                {t(busy === "read" ? "notifications.markAllReadPending" : "notifications.markAllRead")}
              </Button> : null}
              <Button size="sm" variant="outline" aria-busy={busy === "archive"} disabled={busy !== null || deletePending || counts.all === 0} onClick={() => void changeAll("archive")}>
                {busy === "archive" ? <LoaderCircle className="animate-spin" size={14} aria-hidden="true" /> : null}
                {t(busy === "archive" ? "notifications.archiveAllPending" : "notifications.archiveAll")}
              </Button>
            </>}
            <Button size="sm" variant="outline" aria-busy={query.isFetching} disabled={query.isFetching} onClick={() => void query.refetch()}>{t("dude.refresh")}</Button>
          </div>
        </header>
        {query.error || error ? <div role="alert" className="flex flex-wrap items-center gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4 text-sm text-[var(--color-error)]">
          <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">{error ?? query.error?.message}</span>
          <Button size="sm" variant="outline" onClick={() => { setError(null); void query.refetch(); }}>{t("dude.retry")}</Button>
        </div> : null}
        {query.isPending ? <p role="status" className="px-5 py-16 text-center text-sm text-[var(--color-on-surface-variant)]">{t("notifications.loading")}</p> : null}
        <ul className="min-w-0 divide-y divide-[var(--color-outline-variant)]" aria-label={t("notifications.title")}>
          {records.map((item) => <NotificationListItem key={item.id} notification={item} disabled={busy !== null || deletePending} pending={busy}
            onOpen={(trigger) => {
              detailTrigger.current = trigger;
              const next = new URLSearchParams(searchParams);
              setDismissedId(null);
              setDetailReadId(null);
              next.set("notification", item.id);
              setSearchParams(next);
            }}
            onRead={() => void change(item.id, item.readAt === null, false, `read:${item.id}`)}
            onArchive={() => void change(item.id, item.readAt !== null, item.archivedAt === null, `archive:${item.id}`)}
            onDelete={() => setDeleteTarget(item)} />)}
        </ul>
        {!query.isPending && !query.error && records.length === 0 ? <div className="flex min-h-64 flex-col items-center justify-center gap-3 px-5 py-12 text-[var(--color-on-surface-variant)]">
          <Inbox className="size-7 opacity-50" strokeWidth={1.25} aria-hidden="true" />
          <p className="text-sm">{t(filter === "unread" ? "notifications.noUnread" : "notifications.empty")}</p>
        </div> : null}
      </SurfaceCard>
      <NotificationDetailsDialog open={detailsOpen}
        notification={selectedNotification} loading={query.isPending} error={query.error?.message ?? null}
        returnFocus={() => active ? (detailTrigger.current?.isConnected ? detailTrigger.current : toolbar.current?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]') ?? null) : false}
        onClose={() => { if (active) closeDetails(); }}
        onOpenRelated={(path) => { flushSync(() => closeDetails()); navigate(path); }} />
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
