import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchAutomationExecutionPage } from "@/domain/automation/api";
import { listAutomationExecutions } from "@/domain/automation/mock/data";
import type {
  AutomationExecutionListItem,
  AutomationExecutionListQuery,
  AutomationPage,
} from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface AutomationExecutionPageResult {
  loading: boolean;
  page: AutomationPage<AutomationExecutionListItem>;
  refresh: () => void;
}

const executionPageRequests = new Map<string, Promise<AutomationPage<AutomationExecutionListItem>>>();

function parseExecutionQuery(queryKey: string): AutomationExecutionListQuery {
  return JSON.parse(queryKey) as AutomationExecutionListQuery;
}

function loadExecutionPage(
  requestKey: string,
  automationCode: string,
  query: AutomationExecutionListQuery,
): Promise<AutomationPage<AutomationExecutionListItem>> {
  const cached = executionPageRequests.get(requestKey);
  if (cached) return cached;

  const request = fetchAutomationExecutionPage(automationCode, query)
    .catch((error: unknown) => {
      console.warn("[automation] Falling back to mock automation executions.", error);
      return listAutomationExecutions(automationCode, query);
    })
    .finally(() => {
      executionPageRequests.delete(requestKey);
    });

  executionPageRequests.set(requestKey, request);
  return request;
}

function createEmptyExecutionPage(
  query: AutomationExecutionListQuery,
): AutomationPage<AutomationExecutionListItem> {
  return {
    current: Math.max(1, query.current ?? 1),
    records: [],
    size: Math.min(100, Math.max(1, query.size ?? 15)),
    total: 0,
  };
}

export function useAutomationExecutions(
  automationCode: string,
  query: AutomationExecutionListQuery,
): AutomationExecutionPageResult {
  const dataSource = getServiceDataSource("automation");
  const queryKey = JSON.stringify(query);
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(
    () => dataSource === "api" && automationCode.trim().length > 0,
  );
  const [page, setPage] = useState(() => (
    dataSource === "api"
      ? createEmptyExecutionPage(query)
      : listAutomationExecutions(automationCode, query)
  ));

  useEffect(() => {
    if (dataSource === "api") return;
    setLoading(false);
    setPage(listAutomationExecutions(automationCode, parseExecutionQuery(queryKey)));
  }, [automationCode, dataSource, queryKey, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api" || !automationCode.trim()) return;

    let active = true;
    const requestQuery = parseExecutionQuery(queryKey);
    const requestKey = `${automationCode}::${queryKey}::${reloadKey}`;
    setLoading(true);

    loadExecutionPage(requestKey, automationCode, requestQuery)
      .then((nextPage) => {
        if (active) setPage(nextPage);
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [automationCode, dataSource, queryKey, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    loading,
    page,
    refresh,
  };
}
