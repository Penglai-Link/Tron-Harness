import { getPromptVariables } from "@/domain/prompt/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { promptApiEndpoints } from "@/domain/prompt/api";

export function usePromptVariableList() {
  return useServiceQuery({
    domain: "prompt",
    endpoint: promptApiEndpoints.listPromptVariables,
    mockData: () => getPromptVariables(),
    deps: [],
  });
}
