export const autoModelRoutes = {
  TEXT: { code: "Chat", labelKey: "automaticRouters.kinds.Chat" },
  EMBEDDING: { code: "Embedding", labelKey: "automaticRouters.kinds.Embedding" },
  RERANK: { code: "Rerank", labelKey: "automaticRouters.kinds.Rerank" },
};
export function autoModelRouteCode(type: keyof typeof autoModelRoutes) { return `AUTO:${autoModelRoutes[type].code}`; }
