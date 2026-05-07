import { getAutomations } from "@/domain/automation/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { automationApiEndpoints } from "@/domain/automation/api";

export function useAutomations() {
  return useServiceQuery({
    domain: "automation",
    endpoint: automationApiEndpoints.listAutomations,
    mockData: () => getAutomations(),
    deps: [],
  });
}
