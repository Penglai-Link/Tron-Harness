import type { StatusTone } from "./enum";

export interface MetricDatum {
  label: string;
  value: string;
  trend: string;
  tone?: StatusTone;
}

export interface TraceDatum {
  id: string;
  agent: string;
  span: string;
  latency: string;
  status: string;
  time: string;
}

export interface ObservabilityData {
  metrics: MetricDatum[];
  throughput: { label: string; value: number }[];
  errors: { label: string; value: number }[];
  heatmap: number[];
  traces: TraceDatum[];
}
