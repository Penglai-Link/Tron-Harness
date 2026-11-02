export const autoModelRoutes = {
  TEXT: { code: "Chat", labelKey: "automaticRouters.kinds.Chat" },
  TEXT_EMBEDDING: { code: "TextEmbedding", labelKey: "automaticRouters.kinds.TextEmbedding" },
  MULTIMODAL_EMBEDDING: { code: "MultimodalEmbedding", labelKey: "automaticRouters.kinds.MultimodalEmbedding" },
  TEXT_RERANK: { code: "TextRerank", labelKey: "automaticRouters.kinds.TextRerank" },
  MULTIMODAL_RERANK: { code: "MultimodalRerank", labelKey: "automaticRouters.kinds.MultimodalRerank" },
};
export function autoModelRouteCode(type: keyof typeof autoModelRoutes) { return `AUTO:${autoModelRoutes[type].code}`; }
