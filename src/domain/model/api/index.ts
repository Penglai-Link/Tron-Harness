import {
  getModelCostPreview,
  getModels,
} from "@/domain/model/mock/data";
import { useServiceQuery } from "@/domain/service-config";
import type {
  ModelCostLine,
  ModelRegistryItem,
} from "@/domain/model/model/model";

export type {
  ModelCostLine,
  ModelRegistryItem
} from "@/domain/model/model/model";

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
