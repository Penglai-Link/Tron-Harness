export const autoModelRoutes = {
  TEXT: { code: "Chat", labelKey: "modelAutoRoutes.chat" },
  EMBEDDING: { code: "Embedding", labelKey: "modelAutoRoutes.embedding" },
  RERANK: { code: "Rerank", labelKey: "modelAutoRoutes.rerank" },
};
export function autoModelRouteCode(type: keyof typeof autoModelRoutes) { return `AUTO:${autoModelRoutes[type].code}`; }
