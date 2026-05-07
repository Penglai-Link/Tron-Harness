import { getGraphData } from "@/domain/knowledge-graph/mock/data";

import { useServiceQuery } from "@/domain/service-config";
import type { GraphEntity } from "@/domain/knowledge-graph/model/model";

export type {
  GraphEntity
} from "@/domain/knowledge-graph/model/model";

export const knowledgeGraphApiEndpoints = {
  getKnowledgeGraph: "GET /api/v1/knowledge-graph?domain={domain}",
} as const;

export function useGraphData(domain = "customer-support") {
  return useServiceQuery({
    domain: "knowledge-graph",
    endpoint: knowledgeGraphApiEndpoints.getKnowledgeGraph,
    mockData: () => getGraphData(domain),
    deps: [domain],
    params: { domain },
  });
}
