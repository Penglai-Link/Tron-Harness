export interface TraceSummary {
  trigger?: string;
  rootSpanId?: string | null;
  workspaceCode: string;
  traceId: string;
  sessionCode: string;
  sessionName: string | null;
  runCode: string;
  agentCode: string;
  startedAt: number;
  endedAt: number | null;
  durationMs: number;
  status: string;
  spanCount: number;
  errorCount: number;
  modelCount: number;
  toolCount: number;
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
}
export interface TraceSpan {
  spanId: string;
  parentSpanId: string | null;
  name: string;
  startedAt: number;
  durationMs: number;
  status: string;
  attributes: Record<string, string>;
  events: {
    name: string;
    timestamp: number;
    attributes: Record<string, string>;
  }[];
  links: { traceId: string; spanId: string }[];
}
export interface TracePage {
  agentOptions?: { code: string; name: string }[];
  current: number;
  size: number;
  total: number;
  records: TraceSummary[];
}
export interface TraceDetail {
  trace: TraceSummary;
  spans: TraceSpan[];
}
export type TraceSortField = "StartedAt" | "Duration" | "Spans" | "Tokens" | "Errors";
export interface TraceQuery {
  spanId?: string | null;
  keyword?: string;
  status?: string | null;
  agentCode?: string | null;
  keywordAliases?: string[];
  category?: "Page" | "Agent";
  sorts?: { field: TraceSortField; direction: "Asc" | "Desc" }[];
  sortBy?: TraceSortField;
  sortDirection?: "Asc" | "Desc";
  workspaceCode: string;
  sessionCode: string;
  traceId: string;
  current: number;
  size: number;
}

export interface TraceMetrics {
  traceCount: number;
  failedTraceCount: number;
  inputTokens: number;
  outputTokens: number;
  averageDurationMs: number;
  p95DurationMs: number;
}

export interface RuntimeMetrics {
  cpuPercent: number | null;
  memoryBytes: number | null;
  sampledAt: number | null;
  metricPointCount: number;
}
export interface MetricPoint {
  pointId: string;
  name: string;
  description: string;
  unit: string;
  kind: string;
  temporality: number;
  monotonic: boolean;
  timestamp: number;
  resource: Record<string, string>;
  scope: string;
  data: Record<string, unknown>;
}
export interface MetricPage {
  current: number;
  size: number;
  total: number;
  records: MetricPoint[];
}
export interface CollectorConnection {
  endpoint: string;
  authorization: string;
  traceRetentionDays: number;
  metricRetentionDays: number;
  maxTraceRecords: number;
}

export interface ServiceProcess {
  pid: number;
  parentPid: number | null;
  name: string;
  memoryBytes: number;
  startedAt: number;
}
export interface ObservabilityDashboard {
  sampledAt: number;
  traceCount: number;
  failedTraceCount: number;
  runningTraceCount: number;
  inputTokens: number;
  outputTokens: number;
  averageDurationMs: number;
  p95DurationMs: number;
  recentRequests: { startedAt: number; requests: number; errors: number }[];
  cpuPercent: number | null;
  memoryBytes: number | null;
  diskUsage: {
    bytes: number | null;
    sampledAt: number | null;
    paths: string[];
    measurement: "Allocated" | "Logical";
  };
  resourceSampledAt: number | null;
  metricPointCount: number;
  processes: ServiceProcess[];
}
