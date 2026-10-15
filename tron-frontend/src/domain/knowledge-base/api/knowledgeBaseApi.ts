import { requestApi, requestApiBlob } from "@/shared/api/httpClient";
import type { PublishedChunkPage, ChunkConfig, ChunkPreview, KnowledgeDocument, KnowledgeLibrary, KnowledgeLibrarySummary, KnowledgeLibraryDetail, ParsingTier, OcrMode, ParsingCapabilities, IngestionTask } from "../model/KnowledgeLibraryModel";

const root = "/api/v1/knowledge-bases";
const libraryPath = `${root}/{id}`;
const documentPath = `${libraryPath}/documents/{documentId}`;

export function listKnowledgeLibraries(signal?: AbortSignal) {
  return requestApi<KnowledgeLibrarySummary[]>(`GET ${root}`, { signal });
}
export function deleteKnowledgeLibrary(id: string) {
  return requestApi<Record<string, never>>(`DELETE ${libraryPath}`, { pathParams: { id } });
}
export function createKnowledgeLibrary(name: string, description: string) {
  return requestApi<KnowledgeLibrary>(`POST ${root}`, { body: { name, description } });
}
export function getKnowledgeLibrary(id: string, signal?: AbortSignal) {
  return requestApi<KnowledgeLibraryDetail>(`GET ${libraryPath}`, { pathParams: { id }, signal });
}
export function uploadKnowledgeDocument(id: string, file: File) {
  const body = new FormData();
  // getRandomValues also works when the web UI is served on a LAN HTTP origin.
  body.set("idempotencyKey", Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, "0")).join(""));
  body.set("file", file, file.name);
  return requestApi<KnowledgeDocument>(`POST ${libraryPath}/documents`, { pathParams: { id }, body });
}
export function updateKnowledgeDocument(id: string, documentId: string, chunkConfig: ChunkConfig | null, parsingTier?: ParsingTier | null, ocrMode?: OcrMode | null) {
  return requestApi<KnowledgeDocument>(`PATCH ${documentPath}`, { pathParams: { id, documentId }, body: { chunkConfig, ...(parsingTier !== undefined ? { parsingTier } : {}), ...(ocrMode !== undefined ? { ocrMode } : {}) } });
}
export function previewKnowledgeDocument(id: string, documentId: string) {
  return requestApi<ChunkPreview>(`POST ${documentPath}/preview`, { pathParams: { id, documentId }, body: {} });
}
export function indexKnowledgeDocument(id: string, documentId: string, retry = false) {
  return requestApi<KnowledgeDocument>(`POST ${documentPath}/${retry ? "retry" : "index"}`, { pathParams: { id, documentId }, body: {} });
}
export function cancelKnowledgeDocument(id: string, documentId: string) {
  return requestApi<KnowledgeDocument>(`POST ${documentPath}/cancel`, { pathParams: { id, documentId }, body: {} });
}
export function getKnowledgeDocumentTasks(id: string, documentId: string, signal?: AbortSignal) {
  return requestApi<IngestionTask[]>(`GET ${documentPath}/tasks`, { pathParams: { id, documentId }, signal });
}
export function setKnowledgeLibraryGrants(id: string, agentCodes: string[]) {
  return requestApi<unknown>(`PUT ${libraryPath}/grants`, { pathParams: { id }, body: { agentCodes } });
}

export interface AgentKnowledgeLibrary {
  id: string;
  name: string;
  description: string;
  documentCount: number;
  granted: boolean;
}
export function listAgentKnowledgeLibraries(agentCode: string, signal?: AbortSignal) {
  return requestApi<AgentKnowledgeLibrary[]>(`GET ${root}/agent-grants/{agentCode}`, { pathParams: { agentCode }, signal });
}
export function setAgentKnowledgeLibraries(agentCode: string, libraryIds: string[]) {
  return requestApi<Record<string, never>>(`PUT ${root}/agent-grants/{agentCode}`, { pathParams: { agentCode }, body: { libraryIds } });
}

export function getParsingCapabilities(signal?: AbortSignal) {
  return requestApi<ParsingCapabilities>(`GET ${root}/parsing-capabilities`, { signal });
}

export interface DocumentDeletion { documentId: string; state: "DELETED"; originalRetained: boolean; mineruCacheRetained: boolean }
export function deleteKnowledgeDocument(id: string, documentId: string) {
  return requestApi<DocumentDeletion>(`DELETE ${documentPath}`, { pathParams: { id, documentId } });
}

export function getPublishedKnowledgeChunks(id: string, documentId: string, page: number, pageSize: number, signal?: AbortSignal) {
  return requestApi<PublishedChunkPage>(`GET ${documentPath}/chunks`, { pathParams: { id, documentId }, queryParams: { page, pageSize }, signal });
}

export interface PlatformKnowledgeLibrarySummary extends Pick<KnowledgeLibrary, "id" | "name" | "description" | "documentCount"> {
  authorizedAgentCount: number;
}
export function listPlatformKnowledgeLibraries(signal?: AbortSignal) {
  return requestApi<PlatformKnowledgeLibrarySummary[]>(`GET ${root}/platform`, { signal });
}
export function getKnowledgeLibraryAgentGrants(id: string, signal?: AbortSignal) {
  return requestApi<{ agentCodes: string[] }>(`GET ${libraryPath}/agent-grants`, { pathParams: { id }, signal });
}
export function assignKnowledgeLibraryAgents(id: string, agentCodes: string[]) {
  return requestApi<Record<string, never>>(`PUT ${libraryPath}/agent-grants`, { pathParams: { id }, body: { agentCodes } });
}

export function downloadKnowledgeResource(id: string, resourceCode: string, signal?: AbortSignal) {
  return requestApiBlob(`GET ${libraryPath}/resources/{resourceCode}/binary`, { pathParams: { id, resourceCode }, signal });
}
