import { listVariables } from "@/domain/variable/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
export type {
  StatusTone
} from "@/domain/variable/constants/StatusTone";
export type {
  VariableRecord
} from "@/domain/variable/model/VariableModel";

export const variableApiEndpoints = {
  listVariables: "GET /api/v1/variables",
} as const;

export function useVariables() {
  return useServiceQuery({
    domain: "variable",
    endpoint: variableApiEndpoints.listVariables,
    mockData: () => listVariables(),
    deps: [],
  });
}
