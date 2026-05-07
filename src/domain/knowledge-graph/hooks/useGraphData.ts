import { getGraphData } from "@/domain/knowledge-graph/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { knowledgeGraphApiEndpoints } from "@/domain/knowledge-graph/api";

export function useGraphData(domain = "customer-support") {
  return useServiceQuery({
    domain: "knowledge-graph",
    endpoint: knowledgeGraphApiEndpoints.getKnowledgeGraph,
    mockData: () => getGraphData(domain),
    deps: [domain],
    params: { domain },
  });
}
