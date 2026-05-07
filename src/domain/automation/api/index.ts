import { getAutomations } from "@/domain/automation/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
import type { AutomationRecord } from "@/domain/automation/model/AutomationModel";

export type {
  AutomationRecord
} from "@/domain/automation/model/AutomationModel";

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
