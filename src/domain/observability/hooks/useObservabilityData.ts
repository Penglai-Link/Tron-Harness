import { listObservabilityData } from "@/domain/observability/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { observabilityApiEndpoints } from "@/domain/observability/api";

export function useObservabilityData() {
  return useServiceQuery({
    domain: "observability",
    endpoint: observabilityApiEndpoints.getObservabilityOverview,
    mockData: () => listObservabilityData(),
    deps: [],
  });
}
