import {
  getModelCostPreview,
  getModels,
} from "@/domain/model/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import type {
  ModelCostLine,
  ModelRegistryItem,
} from "@/domain/model/model/ModelRegistryModel";

export type {
  ModelCostLine,
  ModelRegistryItem
} from "@/domain/model/model/ModelRegistryModel";

export const modelApiEndpoints = {
  listModels: "GET /api/v1/models",
  getModelCostPreview: "GET /api/v1/models/cost-preview",
} as const;

export function useModelCostPreview() {
  return useServiceQuery({
    domain: "model",
    endpoint: modelApiEndpoints.getModelCostPreview,
    mockData: () => getModelCostPreview(),
    deps: [],
  });
}

export function useModels() {
  return useServiceQuery({
    domain: "model",
    endpoint: modelApiEndpoints.listModels,
    mockData: () => getModels(),
    deps: [],
  });
}
