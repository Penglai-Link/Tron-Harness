import { getAutomations } from "@/domain/automation/mock/data";

import { useServiceQuery } from "@/domain/service-config";
import type { AutomationRecord } from "@/domain/automation/model/model";

export type {
  AutomationRecord
} from "@/domain/automation/model/model";

export const automationApiEndpoints = {
  listAutomations: "GET /api/v1/automations",
} as const;

export function useAutomations() {
  return useServiceQuery({
    domain: "automation",
    endpoint: automationApiEndpoints.listAutomations,
    mockData: () => getAutomations(),
    deps: [],
  });
}
