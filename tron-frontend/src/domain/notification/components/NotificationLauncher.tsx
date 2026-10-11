import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { listNotifications, notificationQueryKey } from "../api/notificationApi";
export function NotificationLauncher() {
  const { t } = useTranslation("pages");
  const query = useQuery({ queryKey: notificationQueryKey, queryFn: ({ signal }) => listNotifications(signal), refetchInterval: 15000 });
  const unread = query.data?.filter(item => item.readAt === null && item.archivedAt === null).length ?? 0;
  return <Button variant="ghost" size="sm" render={<Link to="/notifications" />} aria-label={t("notifications.unreadCount", { count: unread })}>
    {t("notifications.title")}{unread > 0 ? <span className="rounded-full bg-[var(--color-accent)] px-1.5 text-xs text-[var(--color-on-primary)]">{unread}</span> : null}
  </Button>;
}
