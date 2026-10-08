import { requestApi } from "@/shared/api/httpClient";
import type { LoopDesign } from "../model/LoopModel";
const endpoint = "/api/v1/agents/loops";
export function listLoopDesigns(signal?: AbortSignal) {
  return requestApi<LoopDesign[]>(`GET ${endpoint}`, { signal });
}
export function createLoopDesign(design: LoopDesign) {
  const { loopCode, name, kind, nodes, edges, maxParallelNodes } = design;
  return requestApi<LoopDesign>(`POST ${endpoint}`, {
    body: { loopCode, name, kind, nodes, edges, maxParallelNodes },
  });
}
export function updateLoopDesign(design: LoopDesign) {
  const { name, kind, nodes, edges, maxParallelNodes, version } = design;
  return requestApi<LoopDesign>(`PUT ${endpoint}/{loopCode}`, {
    pathParams: { loopCode: design.loopCode },
    body: {
      name,
      kind,
      nodes,
      edges,
      maxParallelNodes,
      expectedVersion: version,
    },
  });
}
export function deleteLoopDesign(loopCode: string) {
  return requestApi(`DELETE ${endpoint}/{loopCode}`, {
    pathParams: { loopCode },
  });
}
