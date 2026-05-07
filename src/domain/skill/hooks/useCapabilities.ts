import { listCapabilities } from "@/domain/skill/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { skillApiEndpoints } from "@/domain/skill/api";

export function useCapabilities() {
  return useServiceQuery({
    domain: "skill",
    endpoint: skillApiEndpoints.listCapabilities,
    mockData: () => listCapabilities(),
    deps: [],
  });
}
