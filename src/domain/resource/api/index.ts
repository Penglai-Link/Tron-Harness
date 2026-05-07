import { listResources } from "@/domain/resource/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
export type {
  StatusTone
} from "@/domain/resource/constants/StatusTone";
export type {
  ResourceRecord
} from "@/domain/resource/model/ResourceModel";

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
