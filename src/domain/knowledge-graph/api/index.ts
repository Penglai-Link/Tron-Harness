import { getGraphData } from "@/domain/knowledge-graph/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
import type { GraphEntity } from "@/domain/knowledge-graph/model/KnowledgeGraphModel";

export type {
  GraphEntity
} from "@/domain/knowledge-graph/model/KnowledgeGraphModel";

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
