import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useTabVisibility } from "@/components/editable-tabs";
import {
  listNotifications,
  notificationQueryKey,
  notificationTarget,
  updateNotification,
} from "../api/notificationApi";
export function NotificationsPage() {
  const { t, i18n } = useTranslation("pages");
  const client = useQueryClient();
  const active = useTabVisibility();
  const [filter, setFilter] = useState<"all" | "unread" | "archived">("all");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const query = useQuery({
    queryKey: notificationQueryKey,
    queryFn: ({ signal }) => listNotifications(signal),
    enabled: active,
    refetchInterval: active ? 15000 : false,
  });
  async function change(id: string, read: boolean, archived: boolean) {
    setBusy(id);
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
  const records = (query.data ?? []).filter((item) =>
    filter === "archived"
      ? item.archivedAt !== null
      : item.archivedAt === null &&
        (filter !== "unread" || item.readAt === null)
  );
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex gap-2">
          {(["all", "unread", "archived"] as const).map((value) => (
            <Button
              key={value}
              size="sm"
              variant={filter === value ? "secondary" : "ghost"}
              aria-pressed={filter === value}
              onClick={() => setFilter(value)}
            >
              {t(`notifications.${value}`)}
            </Button>
          ))}
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={() => void query.refetch()}
        >
          {t("dude.refresh")}
        </Button>
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
              className="flex flex-wrap items-start gap-3 py-4"
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
                {target ? (
                  <Button
                    size="sm"
                    variant="outline"
                    nativeButton={false}
                    render={<Link to={target} />}
                  >
                    {t("notifications.open")}
                  </Button>
                ) : null}
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy === item.id}
                  onClick={() =>
                    void change(
                      item.id,
                      item.readAt === null,
                      item.archivedAt !== null
                    )
                  }
                >
                  {t(
                    item.readAt === null
                      ? "notifications.markRead"
                      : "notifications.markUnread"
                  )}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy === item.id}
                  onClick={() =>
                    void change(
                      item.id,
                      item.readAt !== null,
                      item.archivedAt === null
                    )
                  }
                >
                  {t(
                    item.archivedAt === null
                      ? "notifications.archive"
                      : "notifications.restore"
                  )}
                </Button>
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
    </div>
  );
}
