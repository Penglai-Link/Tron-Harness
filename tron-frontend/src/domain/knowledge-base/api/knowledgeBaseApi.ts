import { requestApi } from "@/shared/api/httpClient";
import type { PublishedChunkPage, ChunkConfig, ChunkPreview, KnowledgeDocument, KnowledgeLibrary, KnowledgeLibrarySummary, KnowledgeLibraryDetail, ParsingTier, OcrMode, ParsingCapabilities } from "../model/KnowledgeLibraryModel";

const root = "/api/v1/knowledge-bases";
const libraryPath = `${root}/{id}`;
const documentPath = `${libraryPath}/documents/{documentId}`;

export function listKnowledgeLibraries(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<KnowledgeLibrarySummary[]>(`GET ${root}`, { queryParams: { workspaceCode }, signal });
}
export function deleteKnowledgeLibrary(workspaceCode: string, id: string) {
  return requestApi<Record<string, never>>(`DELETE ${libraryPath}`, { pathParams: { id }, queryParams: { workspaceCode } });
}
export function createKnowledgeLibrary(workspaceCode: string, name: string, description: string) {
  return requestApi<KnowledgeLibrary>(`POST ${root}`, { body: { workspaceCode, name, description } });
}
export function getKnowledgeLibrary(workspaceCode: string, id: string, signal?: AbortSignal) {
  return requestApi<KnowledgeLibraryDetail>(`GET ${libraryPath}`, { pathParams: { id }, queryParams: { workspaceCode }, signal });
}
export function uploadKnowledgeDocument(workspaceCode: string, id: string, file: File) {
  const body = new FormData();
  body.set("workspaceCode", workspaceCode);
  // getRandomValues also works when the web UI is served on a LAN HTTP origin.
  body.set("idempotencyKey", Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, "0")).join(""));
  body.set("file", file, file.name);
  return requestApi<KnowledgeDocument>(`POST ${libraryPath}/documents`, { pathParams: { id }, body });
}
export function updateKnowledgeDocument(workspaceCode: string, id: string, documentId: string, chunkConfig: ChunkConfig | null, parsingTier?: ParsingTier | null, ocrMode?: OcrMode | null) {
  return requestApi<KnowledgeDocument>(`PATCH ${documentPath}`, { pathParams: { id, documentId }, body: { workspaceCode, chunkConfig, ...(parsingTier !== undefined ? { parsingTier } : {}), ...(ocrMode !== undefined ? { ocrMode } : {}) } });
}
export function previewKnowledgeDocument(workspaceCode: string, id: string, documentId: string) {
  return requestApi<ChunkPreview>(`POST ${documentPath}/preview`, { pathParams: { id, documentId }, body: { workspaceCode } });
}
export function indexKnowledgeDocument(workspaceCode: string, id: string, documentId: string, retry = false) {
  return requestApi<KnowledgeDocument>(`POST ${documentPath}/${retry ? "retry" : "index"}`, { pathParams: { id, documentId }, body: { workspaceCode } });
}
export function cancelKnowledgeDocument(workspaceCode: string, id: string, documentId: string) {
  return requestApi<KnowledgeDocument>(`POST ${documentPath}/cancel`, { pathParams: { id, documentId }, body: { workspaceCode } });
}
export function setKnowledgeLibraryGrants(workspaceCode: string, id: string, agentCodes: string[]) {
  return requestApi<unknown>(`PUT ${libraryPath}/grants`, { pathParams: { id }, body: { workspaceCode, agentCodes } });
}

export interface AgentKnowledgeLibrary {
  id: string;
  name: string;
  description: string;
  documentCount: number;
  granted: boolean;
}
export function listAgentKnowledgeLibraries(workspaceCode: string, agentCode: string, signal?: AbortSignal) {
  return requestApi<AgentKnowledgeLibrary[]>(`GET ${root}/agent-grants/{agentCode}`, { pathParams: { agentCode }, queryParams: { workspaceCode }, signal });
}
export function setAgentKnowledgeLibraries(workspaceCode: string, agentCode: string, libraryIds: string[]) {
  return requestApi<Record<string, never>>(`PUT ${root}/agent-grants/{agentCode}`, { pathParams: { agentCode }, body: { workspaceCode, libraryIds } });
}

export function getParsingCapabilities(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<ParsingCapabilities>(`GET ${root}/parsing-capabilities`, { queryParams: { workspaceCode }, signal });
}

export interface DocumentDeletion { documentId: string; state: "DELETED"; originalRetained: boolean; mineruCacheRetained: boolean }
export function deleteKnowledgeDocument(workspaceCode: string, id: string, documentId: string) {
  return requestApi<DocumentDeletion>(`DELETE ${documentPath}`, { pathParams: { id, documentId }, queryParams: { workspaceCode } });
}

export function getPublishedKnowledgeChunks(workspaceCode: string, id: string, documentId: string, page: number, pageSize: number, signal?: AbortSignal) {
  return requestApi<PublishedChunkPage>(`GET ${documentPath}/chunks`, { pathParams: { id, documentId }, queryParams: { workspaceCode, page, pageSize }, signal });
}
