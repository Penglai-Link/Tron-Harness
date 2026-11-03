import { requestApi } from "@/shared/api/httpClient";
import type {
  TraceQuery,
  TracePage,
  TraceDetail,
  TraceMetrics,
  RuntimeMetrics,
  MetricPage,
  CollectorConnection,
} from "../model/Trace";
export function queryTraces(query: TraceQuery, signal?: AbortSignal) {
  return requestApi<TracePage>("POST /api/v1/observability/traces/query", {
    body: { ...query, workspaceCode: query.workspaceCode || null },
    signal,
  });
}
export function getTrace(
  workspaceCode: string,
  traceId: string,
  signal?: AbortSignal
) {
  return requestApi<TraceDetail>("GET /api/v1/observability/traces/{traceId}", {
    pathParams: { traceId },
    queryParams: { workspaceCode },
    signal,
  });
}

export function getMetrics(
  workspaceCode: string,
  sessionCode: string,
  traceId: string,
  signal?: AbortSignal
) {
  return requestApi<TraceMetrics>("GET /api/v1/observability/metrics", {
    queryParams: { workspaceCode, sessionCode, traceId },
    signal,
  });
}

export function getRuntimeMetrics(signal?: AbortSignal) {
  return requestApi<RuntimeMetrics>("GET /api/v1/observability/runtime", {
    signal,
  });
}
export function queryMetricPoints(
  name: string,
  current: number,
  size: number,
  signal?: AbortSignal,
  workspaceCode?: string
) {
  return requestApi<MetricPage>(
    "POST /api/v1/observability/metric-points/query",
    { body: { workspaceCode: workspaceCode || null, name: name.trim() || null, current, size }, signal }
  );
}
export function getCollectorExportConfig() {
  return requestApi<CollectorConnection>(
    "POST /api/v1/observability/collector/export-config"
  );
}

export function getDashboard(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<import("../model/Trace").ObservabilityDashboard>("GET /api/v1/observability/dashboard", {
    queryParams: { workspaceCode }, signal,
  });
}

export function queryLogs(query: import("../model/ApplicationLog").LogQuery, signal?: AbortSignal) {
  return requestApi<import("../model/ApplicationLog").ApplicationLogPage>("POST /api/v1/observability/logs/query", {
    body: { ...query, workspaceCode: query.workspaceCode || null, beforeLogId: query.beforeLogId ?? null, keyword:query.keyword.trim() || null, level:query.level || null, traceId:query.traceId.trim() || null, spanId:query.spanId.trim() || null }, signal,
  });
}

export function resetObservability() {
  return requestApi<{ resetAt: number }>("POST /api/v1/observability/reset");
}
