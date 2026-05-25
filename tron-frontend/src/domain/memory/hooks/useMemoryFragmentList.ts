import { getMemoryFragments } from "@/domain/memory/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { memoryApiEndpoints } from "@/domain/memory/api";

export function useMemoryFragmentList() {
  return useServiceQuery({
    apiInitialData: () => [],
    domain: "memory",
    endpoint: memoryApiEndpoints.listMemoryFragments,
    mockData: () => getMemoryFragments(),
    deps: [],
  });
}
