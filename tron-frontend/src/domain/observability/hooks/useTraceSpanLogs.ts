import { useInfiniteQuery } from "@tanstack/react-query";
import { queryLogs } from "../api/observabilityApi";
import { dateInTimeZone } from "@/shared/lib/userTimeZone";
import type { TraceSpan } from "../model/Trace";

type Cursor = { date: string; beforeLogId?: string };

/** Page backwards through file positions, then preceding calendar dates. */
export function useTraceSpanLogs(traceId: string, workspaceCode: string, span: TraceSpan, timeZone: string) {
  const firstDate = dateInTimeZone(timeZone, new Date(span.startedAt));
  const lastDate = dateInTimeZone(timeZone, new Date(span.startedAt + Math.max(0, span.durationMs)));
  return useInfiniteQuery({
    queryKey: ["trace-span-logs", traceId, workspaceCode, span.spanId, firstDate, lastDate, timeZone],
    initialPageParam: { date: lastDate } as Cursor,
    queryFn: ({ signal, pageParam }) => queryLogs({
      ...pageParam, workspaceCode, timeZone, traceId, spanId: span.spanId,
      keyword: "", level: "", current: 1, size: 50,
    }, signal),
    getNextPageParam: (page): Cursor | undefined => {
      if (page.total > page.records.length && page.records.length) {
        return { date: page.date, beforeLogId: page.records[page.records.length - 1].logId };
      }
      const previousDate = new Date(Date.parse(`${page.date}T00:00:00Z`) - 86_400_000).toISOString().slice(0, 10);
      return previousDate >= firstDate ? { date: previousDate } : undefined;
    },
    retry: false,
    staleTime: 30_000,
    refetchOnWindowFocus: false,
  });
}
