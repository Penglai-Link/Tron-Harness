import { listResources } from "@/domain/resource/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { resourceApiEndpoints } from "@/domain/resource/api";

export function useResources() {
  return useServiceQuery({
    domain: "resource",
    endpoint: resourceApiEndpoints.listResources,
    mockData: () => listResources(),
    deps: [],
  });
}
