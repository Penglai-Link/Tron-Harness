import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Link, useNavigate } from "react-router";
import { Bell, ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { listNotifications, notificationQueryKey } from "../api/notificationApi";
import { areBrowserNotificationsEnabled, browserNotificationPreferenceChangedEvent } from "../model/browserNotifications";

export function NotificationLauncher() {
  const { t, i18n } = useTranslation("pages");
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const query = useQuery({ queryKey: notificationQueryKey, queryFn: ({ signal }) => listNotifications(signal), refetchInterval: 15000 });
  const notifications = query.data ?? [];
  const unreadItems = notifications.filter(item => item.readAt === null && item.archivedAt === null);
  const unread = unreadItems.length;
  const recentUnread = unreadItems.slice().sort((left, right) => right.createdAt - left.createdAt).slice(0, 5);
  const previousNotificationIds = useRef<Set<string> | null>(null);

  useEffect(() => {
    const currentNotifications = query.data;
    if (!currentNotifications) return;
    const previousIds = previousNotificationIds.current;
    previousNotificationIds.current = new Set(currentNotifications.map((item) => item.id));
    if (!previousIds || !areBrowserNotificationsEnabled() || document.visibilityState === "visible") return;

    for (const item of currentNotifications) {
      if (previousIds.has(item.id) || item.readAt !== null || item.archivedAt !== null) continue;
      try {
        const browserNotification = new Notification(item.title, {
          body: item.body,
          tag: `tron-notification-${item.id}`,
        });
        browserNotification.onclick = () => {
          window.focus();
          navigate(`/notifications?notification=${encodeURIComponent(item.id)}`);
          browserNotification.close();
        };
      } catch {
        // Permission can change between polling and showing the browser notification.
      }
    }
  }, [navigate, query.data]);

  useEffect(() => {
    const refresh = () => query.refetch();
    const eventName = browserNotificationPreferenceChangedEvent();
    window.addEventListener(eventName, refresh);
    return () => window.removeEventListener(eventName, refresh);
  }, [query.refetch]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={<Button className="relative" variant="ghost" size="icon" aria-label={t("notifications.unreadCount", { count: unread })} title={t("notifications.title")} />}
      >
        <Bell size={17} aria-hidden="true" />
        {unread > 0 ? <span className="absolute right-0 top-0 flex min-w-4 translate-x-1/4 -translate-y-1/4 items-center justify-center rounded-full bg-[var(--color-accent)] px-1 text-[10px] leading-4 text-[var(--color-on-primary)]" aria-hidden="true">{unread}</span> : null}
      </PopoverTrigger>
      <PopoverContent side="bottom" align="end" className="w-[min(24rem,calc(100vw-1rem))] p-0">
        <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-3 py-2">
          <p className="text-sm font-medium">{t("notifications.title")}</p>
        </div>
        {query.isPending ? <p className="px-3 py-6 text-center text-sm text-[var(--color-on-surface-variant)]">{t("notifications.loading")}</p> : null}
        {query.error ? <p role="alert" className="px-3 py-4 text-sm text-[var(--color-error)]">{query.error.message}</p> : null}
        {!query.isPending && !query.error && recentUnread.length === 0 ? <p className="px-3 py-6 text-center text-sm text-[var(--color-on-surface-variant)]">{t("notifications.noUnread")}</p> : null}
        {recentUnread.length > 0 ? (
          <ul className="max-h-80 divide-y divide-[var(--color-outline-variant)] overflow-y-auto">
            {recentUnread.map((item) => (
              <li key={item.id}>
                <button
                  type="button"
                  className="block w-full px-3 py-2.5 text-left transition-colors hover:bg-[var(--color-surface-container)] focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-[var(--color-accent)]"
                  onClick={() => { setOpen(false); navigate(`/notifications?notification=${encodeURIComponent(item.id)}`); }}
                >
                  <span className="block truncate text-sm font-medium">{item.title}</span>
                  <span className="mt-1 block line-clamp-2 whitespace-pre-wrap break-words text-xs text-[var(--color-on-surface-variant)]">{item.body}</span>
                  <span className="mt-1 block text-[11px] text-[var(--color-on-surface-variant)]">{new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(item.createdAt)}</span>
                </button>
              </li>
            ))}
          </ul>
        ) : null}
        <div className="flex justify-end border-t border-[var(--color-outline-variant)] p-2">
          <Button size="sm" variant="ghost" nativeButton={false} render={<Link to="/notifications" onClick={() => setOpen(false)} />}>
            <ExternalLink aria-hidden="true" />
            {t("notifications.manage")}
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
