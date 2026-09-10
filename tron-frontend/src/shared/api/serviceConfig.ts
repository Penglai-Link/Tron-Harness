import {
  useEffect,
  useCallback,
  useMemo,
  useRef,
  useState,
  type DependencyList,
} from "react";

import {
  buildApiUrl,
  getApiAccessToken,
  type EndpointParams,
  parseEndpoint,
} from "@/shared/api/httpClient";
import { useToast } from "@/components/ui/toast";

export type ServiceDataSource = "mock" | "api";

export type ServiceDomain =
  | "admin"
  | "agent"
  | "app"
  | "automation"
  | "chat"
  | "knowledge-graph"
  | "mcp"
  | "memory"
  | "model"
  | "observability"
  | "prompt"
  | "rag"
  | "resource"
  | "skill"
  | "task"
  | "variable";

interface ServiceQueryOptions<T> {
  apiInitialData?: () => T;
  domain: ServiceDomain;
  enabled?: boolean;
  endpoint: string;
  mockData: () => T;
  params?: EndpointParams;
  deps?: DependencyList;
  refreshKey?: unknown;
  select?: (payload: unknown) => T;
  suppressErrorToast?: boolean;
}

export interface ServiceQueryState<T> {
  data: T;
  loading: boolean;
  error: Error | null;
  retry: () => void;
}

const domainEnvKeys: Record<ServiceDomain, string> = {
  admin: "ADMIN",
  agent: "AGENT",
  app: "APP",
  automation: "AUTOMATION",
  chat: "CHAT",
  "knowledge-graph": "KNOWLEDGE_GRAPH",
  mcp: "MCP",
  memory: "MEMORY",
  model: "MODEL",
  observability: "OBSERVABILITY",
  prompt: "PROMPT",
  rag: "RAG",
  resource: "RESOURCE",
  skill: "SKILL",
  task: "TASK",
  variable: "VARIABLE",
};

function normalizeDataSource(value: unknown): ServiceDataSource | undefined {
  if (typeof value !== "string") return undefined;
  const normalized = value.trim().toLowerCase();
  return normalized === "api" || normalized === "mock" ? normalized : undefined;
}

function getEnvValue(key: string) {
  return (import.meta.env as Record<string, string | undefined>)[key];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function selectApiData<T>(payload: unknown): T {
  if (!isRecord(payload) || !("code" in payload) || !("data" in payload)) {
    return payload as T;
  }

  if (payload.code !== "SUCCESS" && payload.code !== "0") {
    const message = typeof payload.message === "string" ? payload.message : "API request failed";
    throw new Error(message);
  }

  return payload.data as T;
}

export function selectPagedRecords<T>(payload: unknown): T[] {
  const data = selectApiData<unknown>(payload);
  if (isRecord(data) && Array.isArray(data.records)) return data.records as T[];
  if (Array.isArray(data)) return data as T[];
  return [];
}

export function getServiceDataSource(domain: ServiceDomain): ServiceDataSource {
  const domainSource = normalizeDataSource(getEnvValue(`VITE_TRON_${domainEnvKeys[domain]}_DATA_SOURCE`));
  const defaultSource = normalizeDataSource(getEnvValue("VITE_TRON_DATA_SOURCE"));
  return domainSource ?? defaultSource ?? "mock";
}

function resolveEndpoint(endpoint: string, params?: EndpointParams) {
  const { method, path } = parseEndpoint(endpoint);
  return {
    method,
    url: buildApiUrl(path, params),
  };
}

export function useServiceQueryState<T>({ apiInitialData, domain, enabled = true, endpoint, mockData, params, deps = [], refreshKey = 0, select = selectApiData<T>, suppressErrorToast = false }: ServiceQueryOptions<T>): ServiceQueryState<T> {
  const toast = useToast();
  const dataSource = getServiceDataSource(domain);
  const shouldUseMockData = dataSource === "mock";
  const lastFailureToastKey = useRef<string | null>(null);
  const apiInitialDataRef = useRef(apiInitialData);
  const mockDataRef = useRef(mockData);
  const selectRef = useRef(select);
  const [retryKey, setRetryKey] = useState(0);

  apiInitialDataRef.current = apiInitialData;
  mockDataRef.current = mockData;
  selectRef.current = select;

  const paramsKey = JSON.stringify(params ?? {});
  const request = useMemo(() => resolveEndpoint(endpoint, params), [endpoint, paramsKey]);
  const initialRefreshKey = shouldUseMockData ? refreshKey : undefined;
  const initialValue = useMemo(
    () => (shouldUseMockData ? mockDataRef.current() : apiInitialDataRef.current?.()) as T,
    [endpoint, initialRefreshKey, paramsKey, shouldUseMockData, ...deps],
  );
  const [data, setData] = useState<T>(initialValue);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const retry = useCallback(() => setRetryKey((value) => value + 1), []);

  useEffect(() => {
    lastFailureToastKey.current = null;
    setData(initialValue);
    setError(null);
    setLoading(false);
  }, [initialValue]);

  useEffect(() => {
    if (dataSource !== "api" || !enabled) {
      setLoading(false);
      return;
    }

    const controller = new AbortController();
    const accessToken = getApiAccessToken();
    setLoading(true);
    setError(null);

    fetch(request.url, {
      credentials: "include",
      headers: accessToken
        ? { Authorization: `Bearer ${accessToken}` }
        : undefined,
      method: request.method,
      signal: controller.signal,
    })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`${request.method} ${request.url} failed with ${response.status}`);
        }
        return response.json() as Promise<T>;
      })
      .then((payload) => {
        lastFailureToastKey.current = null;
        setData(selectRef.current(payload));
        setError(null);
        setLoading(false);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        const message = error instanceof Error
          ? error.message
          : `[services] API request failed for ${domain}.`;
        setError(error instanceof Error ? error : new Error(message));
        setLoading(false);
        if (suppressErrorToast) return;
        const toastKey = `${request.method} ${request.url}: ${message}`;
        if (lastFailureToastKey.current === toastKey) return;
        lastFailureToastKey.current = toastKey;
        toast.error(message)
      });

    return () => controller.abort();
  }, [dataSource, domain, enabled, refreshKey, request.method, request.url, retryKey, suppressErrorToast, toast]);

  return { data, loading, error, retry };
}

export function useServiceQuery<T>(options: ServiceQueryOptions<T>): T {
  return useServiceQueryState(options).data;
}
