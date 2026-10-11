import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { fetchPromptPage } from "@/domain/prompt/api";
import { listPlatformPromptsMock } from "@/domain/prompt/mock";
import { usePromptTranslation } from "@/domain/prompt/hooks/usePromptTranslation";
import type { PromptPage, PromptPageQuery, PromptRecord } from "@/domain/prompt/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function emptyPage(query: PromptPageQuery): PromptPage<PromptRecord> {
  return { current: Math.max(1, query.current), records: [], size: Math.max(1, query.size), total: 0 };
}

export function usePromptPage(query: PromptPageQuery, initialPage?: PromptPage<PromptRecord>) {
  const pt = usePromptTranslation();
  const dataSource = getServiceDataSource("prompt");
  const queryKey = useMemo(() => JSON.stringify(query), [query]);
  const previousQueryKey = useRef(queryKey);
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [page, setPage] = useState<PromptPage<PromptRecord>>(() => initialPage ?? emptyPage(query));

  useEffect(() => {
    const requestQuery = JSON.parse(queryKey) as PromptPageQuery;
    const controller = new AbortController();
    if (previousQueryKey.current !== queryKey) {
      setPage(emptyPage(requestQuery));
      previousQueryKey.current = queryKey;
    }
    setError(null);
    setLoading(true);
    const request = dataSource === "api"
      ? fetchPromptPage(requestQuery, controller.signal)
      : Promise.resolve(listPlatformPromptsMock(requestQuery));

    request
      .then((nextPage) => { if (!controller.signal.aborted) setPage(nextPage); })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) setError(reason instanceof Error ? reason : new Error(pt("errors.load", "Prompts could not be loaded")));
      })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });

    return () => controller.abort();
  }, [dataSource, pt, queryKey, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);
  return { error, loading, page, refresh };
}
