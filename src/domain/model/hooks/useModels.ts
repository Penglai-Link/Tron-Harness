import { getModels } from "@/domain/model/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { modelApiEndpoints } from "@/domain/model/api";

export function useModels() {
  return useServiceQuery({
    domain: "model",
    endpoint: modelApiEndpoints.listModels,
    mockData: () => getModels(),
    deps: [],
  });
}
