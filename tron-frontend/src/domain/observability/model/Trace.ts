export interface TraceSummary {
  traceId: string;
  sessionCode: string;
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
  current: number;
  size: number;
  total: number;
  records: TraceSummary[];
}
export interface TraceDetail {
  trace: TraceSummary;
  spans: TraceSpan[];
}
export interface TraceQuery {
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
