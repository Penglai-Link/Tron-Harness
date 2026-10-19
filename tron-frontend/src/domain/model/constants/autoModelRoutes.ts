import type { ProviderRouteCode } from "../model/ModelRegistryModel";

export const autoModelRoutes = {
  TEXT: { code: "ChatAuto", labelKey: "modelAutoRoutes.chat" },
  EMBEDDING: { code: "EmbeddingAuto", labelKey: "modelAutoRoutes.embedding" },
  MULTIMODAL_EMBEDDING: { code: "MultimodalEmbeddingAuto", labelKey: "modelAutoRoutes.multimodalEmbedding" },
  RERANK: { code: "RerankAuto", labelKey: "modelAutoRoutes.rerank" },
} satisfies Record<string, { code: ProviderRouteCode; labelKey: string }>;

export function autoModelRouteCode(type: keyof typeof autoModelRoutes) {
  return `PERSON_AI:ROUTE:${autoModelRoutes[type].code}`;
}
