import {
  useEffect,
  useMemo,
  useState,
  type DependencyList,
} from "react";

export type ServiceDataSource = "mock" | "api";

export type ServiceDomain =
  | "admin"
  | "agent"
  | "app"
  | "automation"
  | "chat"
  | "dashboard"
  | "knowledge-graph"
  | "memory"
  | "model"
  | "observability"
  | "prompt"
  | "resource"
  | "skill"
  | "task"
  | "variable";

type EndpointParams = Record<string, string | number | boolean | undefined>;

interface ServiceQueryOptions<T> {
  domain: ServiceDomain;
  endpoint: string;
  mockData: () => T;
  params?: EndpointParams;
  deps?: DependencyList;
}

const domainEnvKeys: Record<ServiceDomain, string> = {
  admin: "ADMIN",
  agent: "AGENT",
  app: "APP",
  automation: "AUTOMATION",
  chat: "CHAT",
  dashboard: "DASHBOARD",
  "knowledge-graph": "KNOWLEDGE_GRAPH",
  memory: "MEMORY",
  model: "MODEL",
  observability: "OBSERVABILITY",
  prompt: "PROMPT",
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

export function getServiceDataSource(domain: ServiceDomain): ServiceDataSource {
  const domainSource = normalizeDataSource(getEnvValue(`VITE_TRON_${domainEnvKeys[domain]}_DATA_SOURCE`));
  const defaultSource = normalizeDataSource(getEnvValue("VITE_TRON_DATA_SOURCE"));
  return domainSource ?? defaultSource ?? "mock";
}

function resolveApiUrl(path: string, params: EndpointParams = {}) {
  const baseUrl = getEnvValue("VITE_TRON_API_BASE_URL") ?? "";
  const resolvedPath = path.replace(/\{([^}]+)\}/g, (_, key: string) => encodeURIComponent(String(params[key] ?? "")));
  return `${baseUrl}${resolvedPath}`;
}

function parseEndpoint(endpoint: string, params?: EndpointParams) {
  const [method = "GET", path = endpoint] = endpoint.split(/\s+/, 2);
  return {
    method,
    url: resolveApiUrl(path, params),
  };
}

export function useServiceQuery<T>({ domain, endpoint, mockData, params, deps = [] }: ServiceQueryOptions<T>): T {
  const dataSource = getServiceDataSource(domain);
  const mockValue = useMemo(mockData, deps);
  const [data, setData] = useState(mockValue);
  const paramsKey = JSON.stringify(params ?? {});
  const request = useMemo(() => parseEndpoint(endpoint, params), [endpoint, paramsKey]);

  useEffect(() => {
    setData(mockValue);
  }, [mockValue]);

  useEffect(() => {
    if (dataSource !== "api") return;

    const controller = new AbortController();

    fetch(request.url, { method: request.method, signal: controller.signal })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`${request.method} ${request.url} failed with ${response.status}`);
        }
        return response.json() as Promise<T>;
      })
      .then((payload) => {
        setData(payload);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn(`[services] Falling back to mock data for ${domain}.`, error);
      });

    return () => controller.abort();
  }, [dataSource, domain, request.method, request.url]);

  return data;
}
