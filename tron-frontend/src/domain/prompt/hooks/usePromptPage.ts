import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  fetchPlatformPromptPage,
  fetchWorkspacePromptPage,
} from "@/domain/prompt/api";
import {
  listPlatformPromptsMock,
  listWorkspacePromptsMock,
} from "@/domain/prompt/mock";
import { usePromptTranslation } from "@/domain/prompt/hooks/usePromptTranslation";
import type {
  PromptPage,
  PromptPageQuery,
  PromptRecord,
  PromptScopeKind,
} from "@/domain/prompt/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function emptyPage(query: PromptPageQuery): PromptPage<PromptRecord> {
  return {
    current: Math.max(1, query.current),
    records: [],
    size: Math.max(1, query.size),
    total: 0,
  };
}

export function usePromptPage(scopeKind: PromptScopeKind, query: PromptPageQuery) {
  const pt = usePromptTranslation();
  const dataSource = getServiceDataSource("prompt");
  const queryKey = useMemo(() => JSON.stringify(query), [query]);
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [page, setPage] = useState<PromptPage<PromptRecord>>(() => emptyPage(query));

  useEffect(() => {
    const requestQuery = JSON.parse(queryKey) as PromptPageQuery;
    const controller = new AbortController();
    setPage(emptyPage(requestQuery));
    setError(null);

    if (scopeKind === "WORKSPACE" && !requestQuery.workspaceCode) {
      setLoading(false);
      return () => controller.abort();
    }

    setLoading(true);
    const request = scopeKind === "WORKSPACE"
      ? (dataSource === "api"
        ? fetchWorkspacePromptPage(requestQuery as PromptPageQuery & { workspaceCode: string }, controller.signal)
        : Promise.resolve(listWorkspacePromptsMock(requestQuery as PromptPageQuery & { workspaceCode: string })))
      : (dataSource === "api"
        ? fetchPlatformPromptPage(requestQuery, controller.signal)
        : Promise.resolve(listPlatformPromptsMock(requestQuery)));

    request
      .then((nextPage) => {
        if (!controller.signal.aborted) setPage(nextPage);
      })
      .catch((reason: unknown) => {
        if (controller.signal.aborted) return;
        setError(reason instanceof Error ? reason : new Error(pt("errors.load", "Prompts could not be loaded")));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, pt, queryKey, reloadKey, scopeKind]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);
  return { error, loading, page, refresh };
}
