import { Archive, LoaderCircle, MailCheck, MailOpen, RotateCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useUserTimeZone } from "@/shared/hooks/useUserTimeZone";
import { cn } from "@/shared/lib/utils";
import type { PlatformNotification } from "../api/notificationApi";
import { notificationContent, notificationPreview } from "../model/notificationPresentation";

interface NotificationListItemProps {
  notification: PlatformNotification;
  disabled: boolean;
  pending: string | null;
  onOpen: (trigger: HTMLButtonElement) => void;
  onRead: () => void;
  onArchive: () => void;
  onDelete: () => void;
}

export function NotificationListItem({ notification: item, disabled, pending, onOpen, onRead, onArchive, onDelete }: NotificationListItemProps) {
  const { t, i18n } = useTranslation("pages");
  const { timeZone } = useUserTimeZone();
  const { changeKind } = notificationContent(item);
  const unread = item.readAt === null && item.archivedAt === null;
  const date = new Intl.DateTimeFormat(i18n.language, { timeZone, month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(item.createdAt);
  const fullDate = new Intl.DateTimeFormat(i18n.language, { timeZone, dateStyle: "long", timeStyle: "short" }).format(item.createdAt);
  const readLabel = t(item.readAt === null ? "notifications.markRead" : "notifications.markUnread");
  const archiveLabel = t(item.archivedAt === null ? "notifications.archive" : "notifications.restore");
  return (
    <li className="group relative grid min-w-0 grid-cols-[minmax(0,1fr)_auto] gap-x-3 px-4 py-4 transition-colors hover:bg-[var(--color-surface-container)]/40 focus-within:bg-[var(--color-surface-container)]/40 sm:gap-x-6 sm:px-5">
      <button type="button" className="grid min-w-0 grid-cols-[0.5rem_minmax(0,1fr)] gap-3 rounded-sm text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] focus-visible:ring-offset-4 focus-visible:ring-offset-[var(--color-elevated-surface)]"
        aria-label={item.title} onClick={(event) => onOpen(event.currentTarget)}>
        <span className={cn("mt-1.5 size-1.5 rounded-full", unread ? "bg-[var(--color-accent)]" : "bg-transparent")} aria-hidden="true" />
        <span className="flex min-w-0 flex-col gap-1.5">
          <span className={cn("line-clamp-2 break-words text-sm leading-5", unread ? "font-semibold" : "font-medium")}>{item.title}</span>
          <span className="line-clamp-2 text-sm leading-6 text-[var(--color-on-surface-variant)] [overflow-wrap:anywhere]">{notificationPreview(item.body)}</span>
          <span className="mt-0.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs leading-5 text-[var(--color-on-surface-variant)]">
            {unread ? <span className="sr-only">{t("notifications.unread")}</span> : null}
            <span className="max-w-48 truncate">{item.dudeCode ?? t("notifications.platform")}</span>
            {item.resolvedAt !== null ? <Badge variant="outline">{t("notifications.resolved")}</Badge>
              : changeKind ? <Badge variant="secondary">{t(`notifications.changes.${changeKind}`)}</Badge> : null}
            <time className="tabular-nums sm:hidden" dateTime={new Date(item.createdAt).toISOString()} title={fullDate}>{date}</time>
          </span>
        </span>
      </button>
      <div className="flex shrink-0 flex-col items-end justify-between gap-3">
        <time className="hidden text-xs leading-5 whitespace-nowrap text-[var(--color-on-surface-variant)] tabular-nums sm:block" dateTime={new Date(item.createdAt).toISOString()} title={fullDate}>{date}</time>
        <div className="flex items-center gap-0.5 text-[var(--color-on-surface-variant)]">
          {item.archivedAt === null ? <Tooltip>
            <TooltipTrigger render={<Button size="icon-sm" variant="ghost" aria-label={readLabel} aria-busy={pending === `read:${item.id}`} disabled={disabled} onClick={onRead} />}>
              {pending === `read:${item.id}` ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : item.readAt === null ? <MailCheck aria-hidden="true" /> : <MailOpen aria-hidden="true" />}
            </TooltipTrigger>
            <TooltipContent>{readLabel}</TooltipContent>
          </Tooltip> : null}
          <Tooltip>
            <TooltipTrigger render={<Button size="icon-sm" variant="ghost" aria-label={archiveLabel} aria-busy={pending === `archive:${item.id}`} disabled={disabled} onClick={onArchive} />}>
              {pending === `archive:${item.id}` ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : item.archivedAt === null ? <Archive aria-hidden="true" /> : <RotateCw aria-hidden="true" />}
            </TooltipTrigger>
            <TooltipContent>{archiveLabel}</TooltipContent>
          </Tooltip>
          {item.archivedAt !== null ? <Tooltip>
            <TooltipTrigger render={<Button size="icon-sm" variant="ghost" className="text-[var(--color-error)]" aria-label={t("notifications.delete")} disabled={disabled} onClick={onDelete} />}>
              <Trash2 aria-hidden="true" />
            </TooltipTrigger>
            <TooltipContent>{t("notifications.delete")}</TooltipContent>
          </Tooltip> : null}
        </div>
      </div>
    </li>
  );
}
