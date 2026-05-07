import { listObservabilityData } from "@/domain/observability/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
export type {
  StatusTone
} from "@/domain/observability/constants/StatusTone";
export type {
  MetricDatum,
  TraceDatum,
  ObservabilityData
} from "@/domain/observability/model/ObservabilityModel";

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
