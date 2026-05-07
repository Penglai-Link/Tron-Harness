import type { ObservabilityData } from "@/domain/observability/model/ObservabilityModel";

const observabilityData: ObservabilityData = {
  metrics: [
    { label: "Requests", value: "14.2M", trend: "+8.4% from last week", tone: "success" },
    { label: "P95 latency", value: "1.24s", trend: "Stable under failover threshold", tone: "accent" },
    { label: "Error rate", value: "0.08%", trend: "0.02% below budget", tone: "success" },
    { label: "Active traces", value: "342", trend: "42 with open spans", tone: "warning" },
  ],
  throughput: [
    { label: "Mon", value: 48 },
    { label: "Tue", value: 66 },
    { label: "Wed", value: 54 },
    { label: "Thu", value: 78 },
    { label: "Fri", value: 70 },
    { label: "Sat", value: 38 },
    { label: "Sun", value: 44 },
  ],
  errors: [
    { label: "Tool timeout", value: 16 },
    { label: "MCP auth", value: 8 },
    { label: "Prompt guard", value: 4 },
    { label: "Queue retry", value: 12 },
  ],
  heatmap: [22, 42, 68, 35, 80, 58, 48, 76, 31, 64, 54, 88, 39, 71, 46, 62, 28, 57, 74, 44, 69, 51, 33, 60],
  traces: [
    { id: "trc_91A", agent: "Meta-Agent", span: "route.llm.failover", latency: "412ms", status: "Review", time: "2m ago" },
    { id: "trc_88F", agent: "Code Agent", span: "tool.python.execute", latency: "1.8s", status: "Healthy", time: "8m ago" },
    { id: "trc_83D", agent: "Ops Agent", span: "mcp.postgres.query", latency: "242ms", status: "Healthy", time: "16m ago" },
  ],
};

export function listObservabilityData() {
  return observabilityData;
}
