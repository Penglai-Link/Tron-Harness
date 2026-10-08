import { requestApi } from "@/shared/api/httpClient";
import type { RagCorpus, RagSettings, RagTask, RagSearchResult } from "../model/RagModel";
export const listRagCorpora = (signal?: AbortSignal) => requestApi<RagCorpus[]>("GET /api/v1/rag/corpora", { signal });
export const listRagTasks = (signal?: AbortSignal) => requestApi<RagTask[]>("GET /api/v1/rag/tasks", { signal });
export const configureRagCorpus = (id: string, body: RagSettings) => requestApi<RagSettings>("PUT /api/v1/rag/corpora/{id}/settings", { pathParams: { id }, body });
export const submitRagTask = (corpusId: string, documentId?: string) => requestApi<{accepted: boolean}>("POST /api/v1/rag/tasks", { body: { corpusId, documentId } });
export const controlRagTask = (id: string, action: "cancel" | "retry") => requestApi<unknown>(`POST /api/v1/rag/tasks/{id}/${action}`, { pathParams: { id }, body: {} });
export const rebuildRagRecordTask = (id: string) => requestApi<unknown>("POST /api/v1/rag/tasks/{id}/rebuild", { pathParams: { id }, body: {} });
export const searchRag = (body: { query: string; corpusIds: string[]; limit: number }) => requestApi<RagSearchResult>("POST /api/v1/rag/recall", { body });
