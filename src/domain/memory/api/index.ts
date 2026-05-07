import { getMemoryFragments } from "@/domain/memory/mock/data";

import { useServiceQuery } from "@/domain/service-config";
import type { MemoryFragment } from "@/domain/memory/model/model";

export type {
  MemoryFragment
} from "@/domain/memory/model/model";

export const memoryApiEndpoints = {
  listMemoryFragments: "GET /api/v1/memory/fragments",
} as const;

export function useMemoryFragments() {
  return useServiceQuery({
    domain: "memory",
    endpoint: memoryApiEndpoints.listMemoryFragments,
    mockData: () => getMemoryFragments(),
    deps: [],
  });
}
