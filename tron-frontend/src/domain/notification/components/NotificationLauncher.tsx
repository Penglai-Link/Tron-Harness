import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Bell } from "lucide-react";
import { Button } from "@/components/ui/button";
import { listNotifications, notificationQueryKey, notificationTarget } from "../api/notificationApi";
import { areBrowserNotificationsEnabled, browserNotificationPreferenceChangedEvent } from "../model/browserNotifications";
export function NotificationLauncher() {
  const { t } = useTranslation("pages");
  const query = useQuery({ queryKey: notificationQueryKey, queryFn: ({ signal }) => listNotifications(signal), refetchInterval: 15000 });
  const unread = query.data?.filter(item => item.readAt === null && item.archivedAt === null).length ?? 0;
  const previousNotificationIds = useRef<Set<string> | null>(null);

  useEffect(() => {
    const notifications = query.data;
    if (!notifications) return;
    const previousIds = previousNotificationIds.current;
    previousNotificationIds.current = new Set(notifications.map((item) => item.id));
    if (!previousIds || !areBrowserNotificationsEnabled() || document.visibilityState === "visible") return;

    for (const item of notifications) {
      if (previousIds.has(item.id) || item.readAt !== null || item.archivedAt !== null) continue;
      try {
        const browserNotification = new Notification(item.title, {
          body: item.body,
          tag: `tron-notification-${item.id}`,
        });
        browserNotification.onclick = () => {
          window.focus();
          window.location.assign(notificationTarget(item) ?? "/notifications");
          browserNotification.close();
        };
      } catch {
        // Permission can change between polling and showing the notification.
      }
    }
  }, [query.data]);

  useEffect(() => {
    const refresh = () => query.refetch();
    const eventName = browserNotificationPreferenceChangedEvent();
    window.addEventListener(eventName, refresh);
    return () => window.removeEventListener(eventName, refresh);
  }, [query.refetch]);

  return <Button className="relative" variant="ghost" size="icon" nativeButton={false} render={<Link to="/notifications" />} aria-label={t("notifications.unreadCount", { count: unread })} title={t("notifications.title")}>
    <Bell size={17} aria-hidden="true" />
    {unread > 0 ? <span className="absolute right-0 top-0 flex min-w-4 translate-x-1/4 -translate-y-1/4 items-center justify-center rounded-full bg-[var(--color-accent)] px-1 text-[10px] leading-4 text-[var(--color-on-primary)]" aria-hidden="true">{unread}</span> : null}
  </Button>;
}
