import { listObservabilityData } from "@/domain/observability/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { observabilityApiEndpoints } from "@/domain/observability/api";

export function useObservabilityOverview() {
  return useServiceQuery({
    apiInitialData: () => ({
      errors: [],
      heatmap: [],
      metrics: [],
      throughput: [],
      traces: [],
    }),
    domain: "observability",
    endpoint: observabilityApiEndpoints.getObservabilityOverview,
    mockData: () => listObservabilityData(),
    deps: [],
  });
}
