import { requestApi } from "@/shared/api/httpClient";
import type { ChatSession } from "@tron/chat-core";
import type { ChunkConfig } from "../model/KnowledgeLibraryModel";

export interface QualityMetrics { cases: number; recall: number; precision: number; mrr: number; discountedRecall: number; hitRate: number; humanLabeledCases: number; generatedCases: number }
export interface QualityCase { id: string; query: string; partition: "Training" | "Holdout"; provenance: "HumanLabeled" | "Generated"; expected: { documentId: string; span: { blockId: string; start: number; end: number } }[] }
export interface QualityDataset { libraryId: string; sourceFingerprint: string; revision: number; createdAt: number; cases: QualityCase[] }
export interface QualityTrial { strategy: { id: string; version: string }; chunkConfig: { mode: string; maxChars: number; overlapChars: number; headingDepth: number }; training: QualityMetrics | null; holdout: QualityMetrics | null; chunkCount: number; durationMs: number; error: string | null }
export interface QualityRun { id: string; libraryId: string; requestId: string; datasetRevision: number; cases: QualityCase[]; status: string; stage: string; targetRecall: number; topK: number; baselineTraining: QualityMetrics | null; baselineHoldout: QualityMetrics | null; trials: QualityTrial[]; selectedTrial: number | null; published: boolean; targetReached: boolean; missing: { caseId: string; recall: number }[]; modelOperations: number; estimatedInputTokens: number; warnings: string[]; error: string | null; createdAt: number; updatedAt: number }
export interface QualityOptions { candidates: ChunkConfig[]; targetRecall: number; minimumImprovement: number; topK: number; maxTrials: number; maxCases: number; budget: { maxModelOperations: number; maxInputTokens: number; maxDurationSeconds: number } }
export const defaultQualityOptions = (): QualityOptions => ({ candidates: [], targetRecall: .85, minimumImprovement: .02, topK: 8, maxTrials: 3, maxCases: 16, budget: { maxModelOperations: 160, maxInputTokens: 250000, maxDurationSeconds: 900 } });
export const qualityRunActive = (run?: QualityRun) => Boolean(run && ["Queued", "Running", "Publishing", "CancelRequested"].includes(run.status));
const root = "/api/v1/knowledge-bases/{id}/quality";
export const listChunkingStrategies = (signal?: AbortSignal) => requestApi<{ strategies: { id: string; version: string }[] }>("GET /api/v1/knowledge-bases/chunking-strategies", { signal });
export const getQualityDataset = (id: string, signal?: AbortSignal) => requestApi<{ dataset: QualityDataset | null }>(`GET ${root}/dataset`, { pathParams: { id }, signal });
export const putQualityDataset = (id: string, sourceFingerprint: string, cases: QualityCase[]) => requestApi<{ dataset: QualityDataset }>(`PUT ${root}/dataset`, { pathParams: { id }, body: { sourceFingerprint, cases } });
export const listQualityRuns = (id: string, signal?: AbortSignal) => requestApi<{ runs: QualityRun[] }>(`GET ${root}/runs`, { pathParams: { id }, signal });
export const startQualityRun = (id: string, requestId: string, options: QualityOptions) => requestApi<{ run: QualityRun }>(`POST ${root}/runs`, { pathParams: { id }, body: { requestId, options } });
export const cancelQualityRun = (id: string, runId: string) => requestApi<{ run: QualityRun }>(`POST ${root}/runs/{runId}/cancel`, { pathParams: { id, runId }, body: {} });
export async function resolveQualitySession(id: string) {
  const data = await requestApi<{ session: Omit<ChatSession, "createdAt" | "updatedAt"> & { createdAt: number; updatedAt: number } }>(`POST ${root}/session`, { pathParams: { id }, body: {} });
  return { ...data.session, createdAt: new Date(data.session.createdAt).toISOString(), updatedAt: new Date(data.session.updatedAt).toISOString() } satisfies ChatSession;
}
