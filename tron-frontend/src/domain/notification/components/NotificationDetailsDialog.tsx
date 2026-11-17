import { ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useUserTimeZone } from "@/shared/hooks/useUserTimeZone";
import { notificationTarget, type PlatformNotification } from "../api/notificationApi";
import { notificationContent } from "../model/notificationPresentation";
import { NotificationText } from "./NotificationText";

interface NotificationDetailsDialogProps {
  open: boolean;
  notification: PlatformNotification | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
  onOpenRelated: (path: string) => void;
  returnFocus?: () => HTMLElement | null | false;
}

export function NotificationDetailsDialog({ open, notification: item, loading, error, onClose, onOpenRelated, returnFocus }: NotificationDetailsDialogProps) {
  const { t, i18n } = useTranslation("pages");
  const { timeZone } = useUserTimeZone();
  const content = item ? notificationContent(item) : null;
  const target = item ? notificationTarget(item) : null;
  return <Dialog open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
    <DialogContent className="flex max-h-[85dvh] flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl" closeLabel={t("assistant.close")} finalFocus={returnFocus}>
      <DialogHeader className="shrink-0 gap-3 border-b border-[var(--color-outline-variant)] px-5 pt-6 pb-5 sm:px-7">
        <DialogTitle className="pr-6 text-lg font-semibold leading-snug [overflow-wrap:anywhere]">{item?.title ?? t("notifications.title")}</DialogTitle>
        <DialogDescription className="flex flex-wrap items-center gap-x-3 gap-y-2 text-xs leading-5">
          <span>{item?.assistantCode ?? t("notifications.platform")}</span>
          {item ? <time className="tabular-nums" dateTime={new Date(item.createdAt).toISOString()}>{new Intl.DateTimeFormat(i18n.language, { timeZone, dateStyle: "long", timeStyle: "short" }).format(item.createdAt)}</time> : null}
          {content?.changeKind ? <Badge variant="secondary">{t(`notifications.changes.${content.changeKind}`)}</Badge> : null}
          {item?.resolvedAt != null ? <Badge variant="outline">{t("notifications.resolved")}</Badge> : null}
        </DialogDescription>
      </DialogHeader>
      <div key={item?.id ?? "pending"} className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-contain px-5 py-6 sm:px-7">
        {item && content ? <div className="space-y-6">
          <NotificationText text={item.body} />
          {content.requestedAction && item.resolvedAt === null ? <section className="space-y-2 border-l-2 border-[var(--color-accent)] pl-4" aria-label={t("notifications.requestedAction")}>
            <h3 className="text-xs font-semibold text-[var(--color-on-surface-variant)]">{t("notifications.requestedAction")}</h3>
            <NotificationText text={content.requestedAction} />
          </section> : null}
          {content.resolution ? <section className="space-y-2 border-t border-[var(--color-outline-variant)] pt-5" aria-label={t("notifications.resolution")}>
            <h3 className="text-xs font-semibold text-[var(--color-on-surface-variant)]">{t("notifications.resolution")}</h3>
            <NotificationText text={content.resolution} />
          </section> : null}
          {content.evidence.length ? <details className="group border-t border-[var(--color-outline-variant)] pt-4 text-xs text-[var(--color-on-surface-variant)]">
            <summary className="flex w-fit cursor-pointer list-none items-center gap-2 rounded-sm py-1 font-medium outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] [&::-webkit-details-marker]:hidden">
              <ChevronRight className="size-3.5 transition-transform group-open:rotate-90 motion-reduce:transition-none" aria-hidden="true" />
              {t("notifications.evidence")}<span className="tabular-nums">{new Intl.NumberFormat(i18n.language).format(content.evidence.length)}</span>
            </summary>
            <ul className="mt-3 list-disc space-y-3 pl-5 leading-6">{content.evidence.map((entry, index) => <li key={`${index}:${entry}`} className="pl-1 [overflow-wrap:anywhere]">{entry}</li>)}</ul>
          </details> : null}
        </div> : <p role={error ? "alert" : "status"} className="py-6 text-sm text-[var(--color-on-surface-variant)]">{loading ? t("notifications.loading") : error ?? t("notifications.unavailable")}</p>}
      </div>
      <DialogFooter className="m-0 shrink-0 flex-row items-center justify-end gap-2 rounded-none border-[var(--color-outline-variant)] bg-transparent px-5 py-4 sm:px-7">
        <Button variant="outline" onClick={onClose}>{t("assistant.close")}</Button>
        {target ? <Button onClick={() => onOpenRelated(target)}>{t("notifications.openRelated")}</Button> : null}
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
