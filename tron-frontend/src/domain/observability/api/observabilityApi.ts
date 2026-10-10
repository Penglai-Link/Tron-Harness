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
    body: query,
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
  signal?: AbortSignal
) {
  return requestApi<MetricPage>(
    "POST /api/v1/observability/metric-points/query",
    { body: { name: name.trim() || null, current, size }, signal }
  );
}
export function getCollectorExportConfig() {
  return requestApi<CollectorConnection>(
    "POST /api/v1/observability/collector/export-config"
  );
}
