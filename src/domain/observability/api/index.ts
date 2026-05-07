import { listObservabilityData } from "@/domain/observability/mock/data";

import { useServiceQuery } from "@/domain/service-config";
export type {
  StatusTone
} from "@/domain/observability/model/enum";
export type {
  MetricDatum,
  TraceDatum,
  ObservabilityData
} from "@/domain/observability/model/model";

export const observabilityApiEndpoints = {
  getObservabilityOverview: "GET /api/v1/observability/overview",
} as const;

export function useObservabilityData() {
  return useServiceQuery({
    domain: "observability",
    endpoint: observabilityApiEndpoints.getObservabilityOverview,
    mockData: () => listObservabilityData(),
    deps: [],
  });
}
