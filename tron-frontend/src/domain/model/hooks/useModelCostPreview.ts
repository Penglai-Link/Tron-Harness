import { getModelCostPreview } from "@/domain/model/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { modelApiEndpoints } from "@/domain/model/api";

export function useModelCostPreview() {
  return useServiceQuery({
    domain: "model",
    endpoint: modelApiEndpoints.getModelCostPreview,
    mockData: () => getModelCostPreview(),
    deps: [],
  });
}
