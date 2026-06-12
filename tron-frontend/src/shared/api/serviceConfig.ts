import {
  useEffect,
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
import { useToast } from "@/shared/ui/Toast";

export type ServiceDataSource = "mock" | "api";

export type ServiceDomain =
  | "admin"
  | "agent"
  | "app"
  | "automation"
  | "chat"
  | "dashboard"
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
}

const domainEnvKeys: Record<ServiceDomain, string> = {
  admin: "ADMIN",
  agent: "AGENT",
  app: "APP",
  automation: "AUTOMATION",
  chat: "CHAT",
  dashboard: "DASHBOARD",
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

  if (payload.code !== "SUCCESS") {
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

export function useServiceQuery<T>({ apiInitialData, domain, enabled = true, endpoint, mockData, params, deps = [], refreshKey = 0, select = selectApiData<T> }: ServiceQueryOptions<T>): T {
  const toast = useToast();
  const dataSource = getServiceDataSource(domain);
  const shouldUseMockData = dataSource === "mock";
  const lastFailureToastKey = useRef<string | null>(null);
  const apiInitialDataRef = useRef(apiInitialData);
  const mockDataRef = useRef(mockData);
  const selectRef = useRef(select);

  apiInitialDataRef.current = apiInitialData;
  mockDataRef.current = mockData;
  selectRef.current = select;

  const initialValue = useMemo(
    () => (shouldUseMockData ? mockDataRef.current() : apiInitialDataRef.current?.()) as T,
    [shouldUseMockData, ...deps],
  );
  const [data, setData] = useState<T>(initialValue);
  const paramsKey = JSON.stringify(params ?? {});
  const request = useMemo(() => resolveEndpoint(endpoint, params), [endpoint, paramsKey]);

  useEffect(() => {
    lastFailureToastKey.current = null;
    setData(initialValue);
  }, [initialValue]);

  useEffect(() => {
    if (dataSource !== "api" || !enabled) return;

    const controller = new AbortController();
    const accessToken = getApiAccessToken();

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
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        const message = error instanceof Error
          ? error.message
          : `[services] API request failed for ${domain}.`;
        const toastKey = `${request.method} ${request.url}: ${message}`;
        if (lastFailureToastKey.current === toastKey) return;
        lastFailureToastKey.current = toastKey;
        toast.error(message, {
          title: "API request failed",
        });
      });

    return () => controller.abort();
  }, [dataSource, domain, enabled, refreshKey, request.method, request.url, toast]);

  return data;
}
