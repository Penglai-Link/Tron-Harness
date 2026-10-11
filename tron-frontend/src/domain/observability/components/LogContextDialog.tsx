import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { useLocaleFormatters } from "@/shared";
import { queryLogs } from "../api/observabilityApi";
import type { ApplicationLog, LogQuery } from "../model/ApplicationLog";
import { LogMessage } from "./LogMessage";

/** Read physical neighbours, without keyword/level/identifier filters hiding context. */
export function LogContextDialog({ anchor, scope, onClose }: {
  anchor: ApplicationLog;
  scope: Pick<LogQuery, "workspaceCode" | "date" | "timeZone">;
  onClose: () => void;
}) {
  const { t } = useTranslation("observability");
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const [rows, setRows] = useState<ApplicationLog[]>([]);
  const [busy, setBusy] = useState(true);
  const [edges, setEdges] = useState({ before: true, after: true });
  const viewport = useRef<HTMLDivElement>(null);
  const selected = useRef<HTMLDivElement>(null);
  const pending = useRef<{ height: number; top: number } | "center" | null>("center");
  const locked = useRef(true);
  const controller = useRef<AbortController | null>(null);
  const base: LogQuery = { ...scope, keyword: "", level: "", traceId: "", spanId: "", current: 1, size: 50 };
  useEffect(() => {
    const abort = new AbortController();
    controller.current = abort;
    void Promise.all([
      queryLogs({ ...base, beforeLogId: anchor.logId }, abort.signal),
      queryLogs({ ...base, afterLogId: anchor.logId }, abort.signal),
    ]).then(([before, after]) => {
      if (abort.signal.aborted) return;
      setRows([...before.records].reverse().concat(anchor, [...after.records].reverse()));
      setEdges({ before: before.total > before.records.length, after: after.total > after.records.length });
    }).catch(error => {
      if (!abort.signal.aborted) { setRows([anchor]); toast.error(error instanceof Error ? error.message : t("logsLoadFailed")); }
    }).finally(() => { if (!abort.signal.aborted) { locked.current = false; setBusy(false); } });
    return () => abort.abort();
    // The parent keys this component by anchor and query scope.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useLayoutEffect(() => {
    const el = viewport.current;
    if (!el || !rows.length) return;
    if (pending.current === "center" && selected.current) {
      el.scrollTop = selected.current.offsetTop - el.clientHeight / 2 + selected.current.clientHeight / 2;
    } else if (pending.current && pending.current !== "center") {
      el.scrollTop = pending.current.top + el.scrollHeight - pending.current.height;
    }
    pending.current = null;
  }, [rows]);
  const load = async (direction: "before" | "after") => {
    const el = viewport.current;
    if (!el || locked.current || !edges[direction] || !rows.length) return;
    locked.current = true;
    setBusy(true);
    if (direction === "before") pending.current = { height: el.scrollHeight, top: el.scrollTop };
    try {
      const page = await queryLogs({ ...base, [direction === "before" ? "beforeLogId" : "afterLogId"]: direction === "before" ? rows[0].logId : rows[rows.length - 1].logId }, controller.current?.signal);
      if (controller.current?.signal.aborted) return;
      const next = [...page.records].reverse();
      if (pending.current && pending.current !== "center") pending.current.top = el.scrollTop;
      setRows(current => {
        const merged = direction === "before" ? [...next, ...current] : [...current, ...next];
        return [...new Map(merged.map(row => [row.logId, row])).values()];
      });
      setEdges(current => ({ ...current, [direction]: page.total > page.records.length }));
    } catch (error) {
      pending.current = null;
      if (!controller.current?.signal.aborted) toast.error(error instanceof Error ? error.message : t("logsLoadFailed"));
    } finally { locked.current = false; if (!controller.current?.signal.aborted) setBusy(false); }
  };
  return <Dialog open onOpenChange={open => { if (!open) onClose(); }}>
    <DialogContent className="flex h-[min(80vh,900px)] min-h-0 flex-col overflow-hidden sm:max-w-5xl">
      <DialogHeader className="shrink-0 pr-16"><DialogTitle>{t("logContext")}</DialogTitle></DialogHeader>
      <div ref={viewport} role="region" aria-label={t("logContext")} aria-busy={busy} tabIndex={0}
        className="relative min-h-0 flex-1 overflow-auto overscroll-contain rounded-md border bg-[var(--color-elevated-surface)] p-4 font-mono text-xs leading-6"
        onScroll={() => {
          const el = viewport.current;
          if (!el) return;
          if (el.scrollTop < 80) void load("before");
          else if (el.scrollHeight - el.clientHeight - el.scrollTop < 80) void load("after");
        }}>
        {rows.map(row => <div key={row.logId} ref={row.logId === anchor.logId ? selected : undefined}
          data-log-id={row.logId} data-anchor={row.logId === anchor.logId || undefined}
          className={`mb-3 break-words border-l-2 px-3 py-2 ${row.logId === anchor.logId ? "border-foreground bg-muted rounded-r-md ring-1 ring-border" : "border-[var(--color-outline-variant)]"}`}>
          <div className="flex flex-wrap gap-x-3 text-muted-foreground">
            <time>{formatDateTime(row.timestamp, { dateStyle: "short", timeStyle: "medium" })}</time>
            <span className="font-semibold text-foreground">{row.level.toUpperCase()}</span><span className="break-all">{row.target}</span>
          </div>
          <LogMessage message={row.message} continuous />
          {(row.traceId || row.spanId) && <p className="break-all text-muted-foreground">{row.traceId && `trace_id=${row.traceId}  `}{row.spanId && `span_id=${row.spanId}`}</p>}
          {Object.keys(row.attributes).length > 0 && <p className="whitespace-pre-wrap break-all text-muted-foreground">{Object.entries(row.attributes).map(([key,value]) => `${key}=${value}`).join("  ")}</p>}
        </div>)}
      </div>
    </DialogContent>
  </Dialog>;
}
