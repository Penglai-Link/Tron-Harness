import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type DependencyList,
} from "react";

import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useToast } from "@/shared/ui/Toast";

export interface ResourceApiQueryState<T> {
  data: T;
  error: Error | null;
  loading: boolean;
  refresh: () => void;
}

interface ResourceApiQueryOptions<T> {
  api: () => Promise<T>;
  deps: DependencyList;
  enabled?: boolean;
  initialData: () => T;
  mock: () => T;
}

/**
 * Resource pages use POST bodies and the shared requestApi transport, unlike
 * the legacy GET-only service query helper. This keeps the API-mode behavior
 * separate while preserving Mock data for isolated UI tests.
 */
export function useResourceApiQuery<T>({
  api,
  deps,
  enabled = true,
  initialData,
  mock,
}: ResourceApiQueryOptions<T>): ResourceApiQueryState<T> {
  const toast = useToast();
  const dataSource = getServiceDataSource("resource");
  const apiRef = useRef(api);
  const mockRef = useRef(mock);
  const initialDataRef = useRef(initialData);
  const [refreshKey, setRefreshKey] = useState(0);
  const [data, setData] = useState<T>(() => (
    dataSource === "mock" ? mockRef.current() : initialDataRef.current()
  ));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  apiRef.current = api;
  mockRef.current = mock;
  initialDataRef.current = initialData;

  const refresh = useCallback(() => setRefreshKey((value) => value + 1), []);
  const dependencyKey = useMemo(() => JSON.stringify(deps), [deps]);

  useEffect(() => {
    if (dataSource === "mock") {
      setData(mockRef.current());
      setError(null);
      setLoading(false);
      return;
    }
    setData(initialDataRef.current());
  }, [dataSource, dependencyKey]);

  useEffect(() => {
    if (dataSource !== "api" || !enabled) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    apiRef.current()
      .then((next) => {
        if (cancelled) return;
        setData(next);
        setError(null);
      })
      .catch((cause: unknown) => {
        if (cancelled) return;
        const nextError = cause instanceof Error ? cause : new Error("Resource API request failed");
        setError(nextError);
        toast.error(nextError.message);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [dataSource, dependencyKey, enabled, refreshKey, toast]);

  return { data, error, loading, refresh };
}
