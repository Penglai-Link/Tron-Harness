import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import {
  getDashboard,
  queryLogs,
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
    retry: false,
    refetchInterval: 5000,
  });
}
export function useTraceDetail(workspaceCode: string, traceId: string) {
  return useQuery({
    queryKey: ["observability", workspaceCode, traceId],
    queryFn: ({ signal }) => getTrace(workspaceCode, traceId, signal),
    enabled: Boolean(traceId),
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
  const workspaceCode = useWorkspaceSpaces().selectedPrimarySpace?.workspaceCode ?? "";
  const result = useQuery({
    queryKey: ["observability-points", workspaceCode, name, current, size],
    queryFn: ({ signal }) => queryMetricPoints(name, current, size, signal, workspaceCode),
    retry: false,
    refetchInterval: 5000,
  });
  useQueryErrorToast(result.error);
  return result;
}

export function useObservabilityDashboard(workspaceCode: string) {
  const result = useQuery({ queryKey: ["observability-dashboard", workspaceCode],
    queryFn: ({signal}) => getDashboard(workspaceCode, signal),
    retry: false, refetchInterval: 10000,
  });
  useQueryErrorToast(result.error);
  return result;
}

function useLogErrorToast(isError: boolean) {
  const { t } = useTranslation("observability");
  const toast = useToast();
  const notified = useRef(false);
  useEffect(() => {
    if (isError && !notified.current) toast.error(t("logsLoadFailed"));
    notified.current = isError;
  }, [isError, t, toast]);
}

export function useApplicationLogs(query: import("../model/ApplicationLog").LogQuery, live: boolean, enabled = true) {
  const result = useQuery({enabled,queryKey:["observability-logs",query],queryFn:({signal})=>queryLogs(query,signal),retry:false,refetchInterval:live && query.current===1 ? 5000 : false});
  useLogErrorToast(result.isError);
  return result;
}


export function useContinuousLogs(query: Omit<import("../model/ApplicationLog").LogQuery, "current" | "size" | "beforeLogId">, live: boolean) {
  const result = useInfiniteQuery({
    queryKey: ["observability-continuous-logs", query],
    initialPageParam: null as string | null,
    queryFn: ({ signal, pageParam }) => queryLogs({ ...query, current: 1, size: 100, beforeLogId: pageParam }, signal),
    getNextPageParam: (page) => page.total > page.records.length && page.records.length
      ? page.records[page.records.length - 1].logId : undefined,
    retry: false,
    refetchInterval: live ? 5000 : false,
  });
  useLogErrorToast(result.isError);
  return result;
}

export function useQueryErrorToast(error: Error | null) {
  const toast = useToast();
  const notified = useRef<string | null>(null);
  useEffect(() => {
    if (error && notified.current !== error.message) toast.error(error.message);
    notified.current = error?.message ?? null;
  }, [error, toast]);
}
