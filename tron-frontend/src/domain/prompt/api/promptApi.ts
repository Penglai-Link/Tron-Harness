import { requestApi } from "@/shared/api/httpClient";

import type {
  PromptAnalyzeResult,
  PromptCreatePayload,
  PromptDetailPayload,
  PromptDuplicatePayload,
  PromptPage,
  PromptPageQuery,
  PromptRecord,
  PromptTranslation,
  PromptTranslationDraftPayload,
  PromptUpdatePayload,
} from "@/domain/prompt/model";

export const promptApiEndpoints = {
  analyzeTemplate: "POST /api/v1/prompts/analyze-template",
  create: "POST /api/v1/admin/platform/prompts/create",
  delete: "POST /api/v1/admin/platform/prompts/delete",
  detail: "POST /api/v1/admin/platform/prompts/detail",
  duplicate: "POST /api/v1/admin/platform/prompts/duplicate",
  page: "POST /api/v1/admin/platform/prompts/page",
  setEnabled: "POST /api/v1/admin/platform/prompts/set-enabled",
  translateDraft: "POST /api/v1/prompts/translate-draft",
  update: "POST /api/v1/admin/platform/prompts/update",
} as const;

export function fetchPromptPage(payload: PromptPageQuery, signal?: AbortSignal) {
  return requestApi<PromptPage<PromptRecord>>(promptApiEndpoints.page, { body: payload, signal });
}

export function fetchPromptDetail(payload: PromptDetailPayload, signal?: AbortSignal) {
  return requestApi<PromptRecord>(promptApiEndpoints.detail, { body: payload, signal });
}

export function createPrompt(payload: PromptCreatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.create, { body: payload });
}

export function updatePrompt(payload: PromptUpdatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.update, { body: payload });
}

export function duplicatePrompt(payload: PromptDuplicatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.duplicate, { body: payload });
}

export function deletePrompt(payload: PromptDetailPayload) {
  return requestApi<{ promptCode: string }>(promptApiEndpoints.delete, { body: payload });
}

export function setPromptEnabled(payload: PromptDetailPayload & { enabled: boolean; expectedVersion: number }) {
  return requestApi<PromptRecord>(promptApiEndpoints.setEnabled, { body: payload });
}

export function analyzePromptTemplateApi(promptBody: string, signal?: AbortSignal) {
  return requestApi<PromptAnalyzeResult>(promptApiEndpoints.analyzeTemplate, { body: { promptBody }, signal });
}

export function translatePromptDraft(payload: PromptTranslationDraftPayload) {
  return requestApi<PromptTranslation>(promptApiEndpoints.translateDraft, { body: payload });
}
