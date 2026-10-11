import { LogHighlight } from "./LogHighlight";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useLocaleFormatters } from "@/shared";
import { useMotion } from "@eliteforge/agent-ui-motion-react";
import "./continuousLogs.css";
import { useContinuousLogs } from "../hooks/useObservability";
import type { LogQuery } from "../model/ApplicationLog";
import { LogMessage } from "./LogMessage";
import { TraceIdentifier } from "./TraceIdentifier";

export function ContinuousLogs({ filters, live, isToday, onFilter }: {
  filters: Omit<LogQuery, "current" | "size" | "beforeLogId">;
  live: boolean;
  isToday: boolean;
  onFilter: (field: "traceId" | "spanId", value: string) => void;
}) {
  const { t } = useTranslation("observability");
  const { formatDateTime } = useLocaleFormatters();
  const { policy } = useMotion();
  const [following, setFollowing] = useState(true);
  const query = useContinuousLogs(filters, live && isToday && following);
  const viewport = useRef<HTMLDivElement>(null);
  const loadingHistory = useRef(false);
  const previousScroll = useRef<{ height: number; top: number; count: number } | null>(null);
  const rows = useMemo(() => {
    const unique = new Map(query.data?.pages.flatMap(page => page.records).map(row => [row.logId, row]));
    // API order is file position descending, including records sharing a timestamp.
    return [...unique.values()].reverse();
  }, [query.data]);
  useLayoutEffect(() => {
    const element = viewport.current;
    if (!element) return;
    if (previousScroll.current && rows.length > previousScroll.current.count) {
      element.scrollTop = previousScroll.current.top + element.scrollHeight - previousScroll.current.height;
      previousScroll.current = null;
    } else if (following) {
      element.scrollTop = element.scrollHeight;
    }
  }, [rows, following, query.isFetchingNextPage]);
  useLayoutEffect(() => {
    const element = viewport.current;
    if (!element || !following || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => { element.scrollTop = element.scrollHeight; });
    observer.observe(element);
    return () => observer.disconnect();
  }, [following]);
  return <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]">
    <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
      <div className="min-w-0">
        <h2 className="font-semibold">{t("applicationLogs")}</h2>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button variant="ghost" className="log-follow-control" data-active={following} data-live={following && live && isToday} data-motion={policy} aria-pressed={following} onClick={() => setFollowing(!following)}>
          <span className="log-follow-label">{t("followLatestLogs")}</span>
        </Button>
      </div>
    </div>

    <div ref={viewport} role="region" tabIndex={0} aria-label={t("continuousLogs")} className="min-h-0 flex-1 overflow-auto overscroll-contain p-4 font-mono text-xs leading-6"
      onScroll={() => {
        const element = viewport.current;
        if (!element) return;
        const awayFromBottom = element.scrollHeight - element.clientHeight - element.scrollTop > 4;
        setFollowing(!awayFromBottom);
        if (awayFromBottom && element.scrollTop <= 48 && query.hasNextPage && !query.isFetching && !loadingHistory.current) {
          loadingHistory.current = true;
          previousScroll.current = { height: element.scrollHeight, top: element.scrollTop, count: rows.length };
          void query.fetchNextPage()
            .catch(() => { previousScroll.current = null; })
            .finally(() => { loadingHistory.current = false; });
        }
      }}>
      {!rows.length && !query.isPending && !query.isError && <p role="status">{t("noLogs")}</p>}
      {rows.map(row => <div key={row.logId} className="mb-2 break-words border-l-2 border-[var(--color-outline-variant)] pl-3" data-log-id={row.logId}>
        <div className="flex flex-wrap items-baseline gap-x-3 text-muted-foreground">
          <time dateTime={new Date(row.timestamp).toISOString()} className="tabular-nums">{formatDateTime(row.timestamp, { dateStyle: "short", timeStyle: "medium" })}</time>
          <span className={row.level === "Error" ? "font-semibold text-[var(--color-error)]" : row.level === "Warn" ? "font-semibold text-[var(--color-warning)]" : "font-semibold text-foreground"}>{row.level.toUpperCase()}</span>
          <span className="break-all"><LogHighlight text={row.target} keyword={filters.keyword} /></span>
        </div>
        <LogMessage message={row.message} keyword={filters.keyword} continuous />
        {(row.traceId || row.spanId) && <div className="flex flex-wrap gap-x-4">
          {row.traceId && <div className="flex items-center gap-2"><span className="text-muted-foreground">Trace ID</span><TraceIdentifier value={row.traceId} label="Trace ID" onClick={() => onFilter("traceId", row.traceId!)} /></div>}
          {row.spanId && <div className="flex items-center gap-2"><span className="text-muted-foreground">Span ID</span><TraceIdentifier value={row.spanId} label="Span ID" onClick={() => onFilter("spanId", row.spanId!)} /></div>}
        </div>}
        {Object.keys(row.attributes).length > 0 && <p className="whitespace-pre-wrap break-all text-muted-foreground"><LogHighlight text={Object.entries(row.attributes).map(([key, value]) => `${key}=${value}`).join("  ")} keyword={filters.keyword} /></p>}
      </div>)}
    </div>
  </section>;
}
