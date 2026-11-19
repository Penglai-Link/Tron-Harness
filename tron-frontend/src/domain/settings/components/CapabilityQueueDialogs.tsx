import { useTranslation } from "react-i18next";
import { History, ListChecks, LoaderCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import type {
  CapabilityQueueOperation,
  CapabilityQueueSnapshot,
} from "../api/capabilityQueueApi";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  snapshot: CapabilityQueueSnapshot | undefined;
  loading: boolean;
  error: string | null;
  onRetry: () => void;
}

function useQueueTime() {
  const { formatDateTime } = useLocaleFormatters();
  return (value: number) =>
    formatDateTime(value, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
}

export function CapabilityExecutionQueueDialog({
  open,
  onOpenChange,
  snapshot,
  loading,
  error,
  onRetry,
}: Props) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`capabilityCenter.queue.${key}`);
  const time = useQueueTime();
  const operationLabel = (operation: CapabilityQueueOperation) =>
    t(
      `capabilityCenter.${
        operation === "enable"
          ? "enable"
          : operation === "disable"
          ? "disable"
          : "uninstall"
      }`
    );
  const items = snapshot?.items ?? [];
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ListChecks size={16} aria-hidden="true" />
            {text("title")}
          </DialogTitle>
          <DialogDescription>{text("description")}</DialogDescription>
        </DialogHeader>
        {snapshot?.agentBusy && items.length > 0 ? (
          <Badge variant="warning" className="w-fit">
            {text("agentBusy")}
          </Badge>
        ) : null}
        {loading ? (
          <p
            role="status"
            className="flex items-center gap-2 text-sm text-[var(--color-on-surface-variant)]"
          >
            <LoaderCircle
              size={14}
              className="animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
            {text("loading")}
          </p>
        ) : error ? (
          <div role="alert" className="space-y-2">
            <p className="text-sm text-[var(--color-error)]">
              {text("loadFailed")}: {error}
            </p>
            <Button variant="outline" size="sm" onClick={onRetry}>
              {text("retry")}
            </Button>
          </div>
        ) : items.length === 0 ? (
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {text("empty")}
          </p>
        ) : (
          <ol className="divide-y divide-[var(--color-outline-variant)]">
            {items.map((item, index) => (
              <li
                key={item.id}
                className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 py-2 text-sm"
              >
                <span className="w-6 shrink-0 text-xs tabular-nums text-[var(--color-on-surface-variant)]">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span className="min-w-0 flex-1 break-words">
                  {operationLabel(item.operation)} {item.packName}
                </span>
                <Badge variant={item.status === "running" ? "info" : "outline"}>
                  {text(item.status === "running" ? "running" : "waiting")}
                </Badge>
                <time
                  className="text-xs tabular-nums text-[var(--color-on-surface-variant)]"
                  dateTime={new Date(item.createdAt).toISOString()}
                >
                  {time(item.createdAt)}
                </time>
              </li>
            ))}
          </ol>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("capabilityCenter.close")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function CapabilityOperationLogDialog({
  open,
  onOpenChange,
  snapshot,
  loading,
  error,
  onRetry,
}: Props) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`capabilityCenter.queue.${key}`);
  const time = useQueueTime();
  const operationLabel = (operation: CapabilityQueueOperation) =>
    t(
      `capabilityCenter.${
        operation === "enable"
          ? "enable"
          : operation === "disable"
          ? "disable"
          : "uninstall"
      }`
    );
  const log = snapshot?.log ?? [];
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <History size={16} aria-hidden="true" />
            {text("log")}
          </DialogTitle>
          <DialogDescription>{text("logDescription")}</DialogDescription>
        </DialogHeader>
        {loading ? (
          <p
            role="status"
            className="flex items-center gap-2 text-sm text-[var(--color-on-surface-variant)]"
          >
            <LoaderCircle
              size={14}
              className="animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
            {text("loading")}
          </p>
        ) : error ? (
          <div role="alert" className="space-y-2">
            <p className="text-sm text-[var(--color-error)]">
              {text("loadFailed")}: {error}
            </p>
            <Button variant="outline" size="sm" onClick={onRetry}>
              {text("retry")}
            </Button>
          </div>
        ) : log.length === 0 ? (
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {text("logEmpty")}
          </p>
        ) : (
          <ul className="divide-y divide-[var(--color-outline-variant)]">
            {log.map((entry) => (
              <li key={entry.id} className="flex min-w-0 flex-col gap-1 py-2">
                <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-sm">
                  <Badge variant={entry.succeeded ? "success" : "error"}>
                    {text(entry.succeeded ? "succeeded" : "failed")}
                  </Badge>
                  <span className="min-w-0 flex-1 break-words">
                    {operationLabel(entry.operation)} {entry.packName}
                  </span>
                  <time
                    className="text-xs tabular-nums text-[var(--color-on-surface-variant)]"
                    dateTime={new Date(entry.finishedAt).toISOString()}
                  >
                    {time(entry.finishedAt)}
                  </time>
                </div>
                {entry.error ? (
                  <p
                    className="break-words text-xs leading-5 text-[var(--color-error)]"
                    title={entry.error}
                  >
                    {entry.error}
                  </p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("capabilityCenter.close")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
