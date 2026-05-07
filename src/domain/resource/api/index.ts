import { listResources } from "@/domain/resource/mock/data";

import { useServiceQuery } from "@/domain/service-config";
export type {
  StatusTone
} from "@/domain/resource/model/enum";
export type {
  ResourceRecord
} from "@/domain/resource/model/model";

export const resourceApiEndpoints = {
  listResources: "GET /api/v1/resources",
} as const;

export function useResources() {
  return useServiceQuery({
    domain: "resource",
    endpoint: resourceApiEndpoints.listResources,
    mockData: () => listResources(),
    deps: [],
  });
}
