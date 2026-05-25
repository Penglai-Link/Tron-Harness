import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchAutomationPage } from "@/domain/automation/api";
import { listAutomations } from "@/domain/automation/mock/data";
import type {
  AutomationListItem,
  AutomationListQuery,
  AutomationPage,
} from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface AutomationListResult {
  loading: boolean;
  page: AutomationPage<AutomationListItem>;
  refresh: () => void;
}

const automationListRequests = new Map<string, Promise<AutomationPage<AutomationListItem>>>();

function createEmptyAutomationPage(
  query: AutomationListQuery,
): AutomationPage<AutomationListItem> {
  return {
    current: Math.max(1, query.current ?? 1),
    records: [],
    size: Math.min(100, Math.max(1, query.size ?? 15)),
    total: 0,
  };
}

function parseAutomationListQuery(queryKey: string): AutomationListQuery {
  return JSON.parse(queryKey) as AutomationListQuery;
}

function loadAutomationPage(
  requestKey: string,
  query: AutomationListQuery,
): Promise<AutomationPage<AutomationListItem>> {
  const cached = automationListRequests.get(requestKey);
  if (cached) return cached;

  const request = fetchAutomationPage(query)
    .catch((error: unknown) => {
      console.warn("[automation] Automation list request failed.", error);
      return {
        current: Math.max(1, query.current ?? 1),
        records: [],
        size: Math.min(100, Math.max(1, query.size ?? 15)),
        total: 0,
      };
    })
    .finally(() => {
      automationListRequests.delete(requestKey);
    });

  automationListRequests.set(requestKey, request);
  return request;
}

export function useAutomationList(query: AutomationListQuery): AutomationListResult {
  const dataSource = getServiceDataSource("automation");
  const hasWorkspaceCode = Boolean(query.workspaceCode.trim());
  const queryKey = JSON.stringify(query);
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(() => (
    dataSource === "api" ? createEmptyAutomationPage(query) : listAutomations(query)
  ));

  useEffect(() => {
    if (dataSource !== "api") {
      setPage(listAutomations(parseAutomationListQuery(queryKey)));
      return;
    }

    if (!hasWorkspaceCode) {
      setPage(createEmptyAutomationPage(parseAutomationListQuery(queryKey)));
    }
  }, [dataSource, hasWorkspaceCode, queryKey, reloadKey]);

  useEffect(() => {
    if (!hasWorkspaceCode) {
      setLoading(false);
      return;
    }
    if (dataSource !== "api") return;

    let active = true;
    const requestQuery = parseAutomationListQuery(queryKey);
    const requestKey = `${queryKey}::${reloadKey}`;
    setLoading(true);

    loadAutomationPage(requestKey, requestQuery)
      .then((nextPage) => {
        if (active) setPage(nextPage);
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [dataSource, hasWorkspaceCode, queryKey, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    loading,
    page,
    refresh,
  };
}
