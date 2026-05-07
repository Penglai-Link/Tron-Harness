import { listCapabilities } from "@/domain/skill/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { skillApiEndpoints } from "@/domain/skill/api";

export function useCapabilityList() {
  return useServiceQuery({
    domain: "skill",
    endpoint: skillApiEndpoints.listCapabilities,
    mockData: () => listCapabilities(),
    deps: [],
  });
}
