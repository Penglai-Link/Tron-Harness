import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import { DEFAULT_TIME_ZONE } from "@/shared/lib/userTimeZone";
import type { TraceSpan } from "../model/Trace";
import type { LogLevel } from "../model/ApplicationLog";
import { useTraceSpanLogs } from "../hooks/useTraceSpanLogs";
import { eventErrorMessage, spanErrorMessage } from "./traceError";
import { LogMessage } from "./LogMessage";

interface LogEntry {
  id: string;
  timestamp: number;
  level: LogLevel;
  target: string;
  message: string;
  attributes: Record<string, string>;
  source: "span" | "application";
}

export function TraceSpanLogs({ traceId, workspaceCode, span }: {
  traceId: string;
  workspaceCode: string;
  span: TraceSpan;
}) {
  const { t } = useTranslation("observability");
  const { timeZone = DEFAULT_TIME_ZONE, formatDateTime } = useLocaleFormatters();
  const query = useTraceSpanLogs(traceId, workspaceCode, span, timeZone);
  const [expanded, setExpanded] = useState<string | null>(null);
  const entries = useMemo(() => {
    const records: LogEntry[] = [];
    for (const [index, event] of span.events.entries()) {
      const error = eventErrorMessage(event);
      const message = error || event.attributes["log.message"];
      if (!message) continue;
      const severity = event.attributes["log.severity"]?.toLowerCase();
      const level: LogLevel = error ? "Error" : severity === "warn" || severity === "warning" ? "Warn" : severity === "error" ? "Error" : "Info";
      records.push({ id: `span:${index}`, timestamp: event.timestamp, level, target: event.name, message, attributes: event.attributes, source: "span" });
    }
    const message = spanErrorMessage(span, t) || (span.status === "Error" ? t("failureReasonUnavailable") : null);
    if (message && !records.some(record => record.message === message)) {
      records.push({ id: "span:outcome", timestamp: span.startedAt + span.durationMs,
        level: span.status === "Error" ? "Error" : "Info", target: span.attributes["error.type"] || span.name,
        message, attributes: span.attributes, source: "span" });
    }
    const logs = new Map(query.data?.pages.flatMap(page => page.records).map(log => [log.logId, log]));
    records.push(...[...logs.values()].map(log => ({ ...log, id: `application:${log.logId}`, source: "application" as const })));
    return records.sort((first, second) => second.timestamp - first.timestamp);
  }, [span, query.data, t]);

  return <section aria-label={t("spanLogs")} className="min-w-0">
    <ol className="min-w-0 divide-y divide-[var(--color-outline-variant)]">
      {entries.map(entry => <li key={entry.id} className="min-w-0 py-3 first:pt-1">
        <div className="mb-2 flex flex-wrap items-baseline gap-x-2 gap-y-1 text-xs text-[var(--color-on-surface-variant)]">
          <time dateTime={new Date(entry.timestamp).toISOString()} className="font-mono tabular-nums">{formatDateTime(entry.timestamp, { dateStyle: "short", timeStyle: "medium" })}</time>
          <span className={`font-mono font-semibold ${entry.level === "Error" ? "text-[var(--color-error)]" : entry.level === "Warn" ? "text-[var(--color-warning)]" : ""}`}>{entry.level.toUpperCase()}</span>
          <span>{t(entry.source === "span" ? "spanLogSource" : "applicationLogs")}</span>
        </div>
        <p className="mb-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{entry.target}</p>
        <LogMessage message={entry.message} continuous />
        {Object.keys(entry.attributes).length > 0 && <>
          <Button variant="link" size="sm" className="mt-1 h-auto p-0 text-xs" aria-expanded={expanded === entry.id}
            onClick={() => setExpanded(expanded === entry.id ? null : entry.id)}>{t(expanded === entry.id ? "showLess" : "logDetails")}</Button>
          {expanded === entry.id && <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap rounded bg-[var(--color-surface-container)] p-3 font-mono text-xs [overflow-wrap:anywhere]">{JSON.stringify(entry.attributes, null, 2)}</pre>}
        </>}
      </li>)}
    </ol>
    {query.isPending && <p role="status" className="py-3 text-xs text-[var(--color-on-surface-variant)]">{t("loading")}</p>}
    {!entries.length && !query.isPending && !query.isError && <p role="status" className="py-8 text-center text-sm text-[var(--color-on-surface-variant)]">{t("noLogs")}</p>}
    {query.isError && <div role="alert" className="flex items-center justify-between gap-3 py-3 text-xs">
      <span>{t("logsLoadFailed")}</span><Button variant="ghost" size="sm" onClick={() => void query.refetch()}>{t("retry")}</Button>
    </div>}
    {query.hasNextPage && <Button variant="ghost" size="sm" className="mt-3 w-full" disabled={query.isFetchingNextPage}
      onClick={() => void query.fetchNextPage()}>{t(query.isFetchingNextPage ? "loading" : "loadEarlierLogs")}</Button>}
  </section>;
}
