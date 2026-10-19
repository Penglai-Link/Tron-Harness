import { requestApi } from "@/shared/api/httpClient";
import type { RagGraphBrowseRequest, RagGraphSearchRequest, RagGraphExpandRequest, RagGraphWindow } from "../model/RagGraphModel";
export const browseRagGraph = (body: RagGraphBrowseRequest, signal?: AbortSignal) => requestApi<RagGraphWindow>("POST /api/v1/rag/graph/browse", { body, signal });
export const searchRagGraph = (body: RagGraphSearchRequest, signal?: AbortSignal) => requestApi<RagGraphWindow>("POST /api/v1/rag/graph/search", { body, signal });
export const expandRagGraph = (body: RagGraphExpandRequest, signal?: AbortSignal) => requestApi<RagGraphWindow>("POST /api/v1/rag/graph/expand", { body, signal });
