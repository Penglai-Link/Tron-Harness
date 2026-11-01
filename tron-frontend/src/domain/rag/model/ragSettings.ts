import { autoModelRouteCode } from "@/domain/model/constants/autoModelRoutes";
import type { RagSettings } from "./RagModel";

export function defaultRagSettings(): RagSettings {
  return { fulltext: true, vector: true, graph: true, embeddingDimension: 1024,
    embeddingModelCode: autoModelRouteCode("MULTIMODAL_EMBEDDING"),
    graphModelCode: autoModelRouteCode("TEXT"),
    rerankModelCode: autoModelRouteCode("TEXT_RERANK"), imageDescriptionModelCode: null };
}
export function canSaveRagSettings(settings: RagSettings) {
  return (settings.fulltext || settings.vector || settings.graph)
    && (!(settings.vector || settings.graph) || !!settings.embeddingModelCode)
    && (!settings.graph || !!settings.graphModelCode);
}
