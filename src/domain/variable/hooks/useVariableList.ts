import { listVariables } from "@/domain/variable/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { variableApiEndpoints } from "@/domain/variable/api";

export function useVariableList() {
  return useServiceQuery({
    domain: "variable",
    endpoint: variableApiEndpoints.listVariables,
    mockData: () => listVariables(),
    deps: [],
  });
}
