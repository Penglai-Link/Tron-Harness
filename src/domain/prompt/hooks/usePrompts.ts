import { listPrompts } from "@/domain/prompt/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { promptApiEndpoints } from "@/domain/prompt/api";

export function usePrompts() {
  return useServiceQuery({
    domain: "prompt",
    endpoint: promptApiEndpoints.listPrompts,
    mockData: () => listPrompts(),
    deps: [],
  });
}
