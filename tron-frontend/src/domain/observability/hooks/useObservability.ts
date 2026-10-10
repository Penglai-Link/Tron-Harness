import { useQuery } from "@tanstack/react-query";
import {
  queryTraces,
  getTrace,
  getMetrics,
  getRuntimeMetrics,
  queryMetricPoints,
} from "../api/observabilityApi";
import type { TraceQuery } from "../model/Trace";
export function useTraceList(query: TraceQuery) {
  return useQuery({
    queryKey: ["observability", query],
    queryFn: ({ signal }) => queryTraces(query, signal),
    enabled: Boolean(query.workspaceCode),
    retry: false,
    refetchInterval: 5000,
  });
}
export function useTraceDetail(workspaceCode: string, traceId: string) {
  return useQuery({
    queryKey: ["observability", workspaceCode, traceId],
    queryFn: ({ signal }) => getTrace(workspaceCode, traceId, signal),
    enabled: Boolean(workspaceCode && traceId),
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.trace.status === "Running" ? 2000 : false,
  });
}

export function useTraceMetrics(
  workspaceCode: string,
  sessionCode: string,
  traceId: string
) {
  return useQuery({
    queryKey: ["observability-metrics", workspaceCode, sessionCode, traceId],
    queryFn: ({ signal }) =>
      getMetrics(workspaceCode, sessionCode, traceId, signal),
    enabled: Boolean(workspaceCode),
    retry: false,
    refetchInterval: 5000,
  });
}

export function useRuntimeMetrics() {
  return useQuery({
    queryKey: ["observability-runtime"],
    queryFn: ({ signal }) => getRuntimeMetrics(signal),
    retry: false,
    refetchInterval: 5000,
  });
}
export function useMetricPoints(name: string, current: number, size: number) {
  return useQuery({
    queryKey: ["observability-points", name, current, size],
    queryFn: ({ signal }) => queryMetricPoints(name, current, size, signal),
    retry: false,
    refetchInterval: 5000,
  });
}
