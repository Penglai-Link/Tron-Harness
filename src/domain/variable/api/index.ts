import { listVariables } from "@/domain/variable/mock/data";

import { useServiceQuery } from "@/domain/service-config";
export type {
  StatusTone
} from "@/domain/variable/model/enum";
export type {
  VariableRecord
} from "@/domain/variable/model/model";

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
