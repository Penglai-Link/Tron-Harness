import { getMemoryFragments } from "@/domain/memory/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { memoryApiEndpoints } from "@/domain/memory/api";

export function useMemoryFragments() {
  return useServiceQuery({
    domain: "memory",
    endpoint: memoryApiEndpoints.listMemoryFragments,
    mockData: () => getMemoryFragments(),
    deps: [],
  });
}
