import { autoModelRouteCode } from "@/domain/model/constants/autoModelRoutes";
import type { RagSettings } from "./RagModel";

export function defaultRagSettings(): RagSettings {
  return { fulltext: true, vector: true, graph: false, embeddingDimension: 1024,
    embeddingModelCode: autoModelRouteCode("EMBEDDING"),
    graphModelCode: autoModelRouteCode("TEXT"),
    rerankModelCode: autoModelRouteCode("RERANK"), imageDescriptionModelCode: null };
}
export function canSaveRagSettings(settings: RagSettings) {
  return (settings.fulltext || settings.vector || settings.graph)
    && (!(settings.vector || settings.graph) || !!settings.embeddingModelCode)
    && (!settings.graph || !!settings.graphModelCode);
}
