import { requestApi } from "@/shared/api/httpClient";

import type {
  PlatformPromptCreatePayload,
  PlatformPromptUpdatePayload,
  PromptAnalyzeResult,
  PromptDetailPayload,
  PromptDuplicatePayload,
  PromptExportResult,
  PromptPage,
  PromptPageQuery,
  PromptRecord,
  WorkspacePromptCreatePayload,
  WorkspacePromptUpdatePayload,
} from "@/domain/prompt/model";

export const promptApiEndpoints = {
  analyzeTemplate: "POST /api/v1/prompts/analyze-template",
  createPlatform: "POST /api/v1/admin/platform/prompts/create",
  createWorkspace: "POST /api/v1/prompts/workspace-create",
  deletePlatform: "POST /api/v1/admin/platform/prompts/delete",
  deleteWorkspace: "POST /api/v1/prompts/workspace-delete",
  detailPlatform: "POST /api/v1/admin/platform/prompts/detail",
  detailWorkspace: "POST /api/v1/prompts/workspace-detail",
  duplicatePlatform: "POST /api/v1/admin/platform/prompts/duplicate",
  duplicateWorkspace: "POST /api/v1/prompts/workspace-duplicate",
  pagePlatform: "POST /api/v1/admin/platform/prompts/page",
  pageWorkspace: "POST /api/v1/prompts/workspace-page",
  revertWorkspace: "POST /api/v1/prompts/workspace-revert",
  setAllowInheritanceWorkspace: "POST /api/v1/prompts/workspace-set-allow-inheritance",
  setEnabledPlatform: "POST /api/v1/admin/platform/prompts/set-enabled",
  setEnabledWorkspace: "POST /api/v1/prompts/workspace-set-enabled",
  updatePlatform: "POST /api/v1/admin/platform/prompts/update",
  updateWorkspace: "POST /api/v1/prompts/workspace-update",
  workspaceExport: "POST /api/v1/prompts/workspace-export",
} as const;

export function fetchWorkspacePromptPage(payload: PromptPageQuery & { workspaceCode: string }, signal?: AbortSignal) {
  return requestApi<PromptPage<PromptRecord>>(promptApiEndpoints.pageWorkspace, { body: payload, signal });
}

export function fetchPlatformPromptPage(payload: PromptPageQuery, signal?: AbortSignal) {
  return requestApi<PromptPage<PromptRecord>>(promptApiEndpoints.pagePlatform, { body: payload, signal });
}

export function fetchWorkspacePromptDetail(payload: PromptDetailPayload & { workspaceCode: string }, signal?: AbortSignal) {
  return requestApi<PromptRecord>(promptApiEndpoints.detailWorkspace, { body: payload, signal });
}

export function fetchPlatformPromptDetail(payload: PromptDetailPayload, signal?: AbortSignal) {
  return requestApi<PromptRecord>(promptApiEndpoints.detailPlatform, { body: payload, signal });
}

export function createWorkspacePrompt(payload: WorkspacePromptCreatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.createWorkspace, { body: payload });
}

export function createPlatformPrompt(payload: PlatformPromptCreatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.createPlatform, { body: payload });
}

export function updateWorkspacePrompt(payload: WorkspacePromptUpdatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.updateWorkspace, { body: payload });
}

export function updatePlatformPrompt(payload: PlatformPromptUpdatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.updatePlatform, { body: payload });
}

export function duplicateWorkspacePrompt(payload: PromptDuplicatePayload & { workspaceCode: string }) {
  return requestApi<PromptRecord>(promptApiEndpoints.duplicateWorkspace, { body: payload });
}

export function duplicatePlatformPrompt(payload: PromptDuplicatePayload) {
  return requestApi<PromptRecord>(promptApiEndpoints.duplicatePlatform, { body: payload });
}

export function deleteWorkspacePrompt(payload: PromptDetailPayload & { workspaceCode: string }) {
  return requestApi<{ promptCode: string }>(promptApiEndpoints.deleteWorkspace, { body: payload });
}

export function deletePlatformPrompt(payload: PromptDetailPayload) {
  return requestApi<{ promptCode: string }>(promptApiEndpoints.deletePlatform, { body: payload });
}

export function revertWorkspacePrompt(payload: PromptDetailPayload & { workspaceCode: string }) {
  return requestApi<PromptRecord>(promptApiEndpoints.revertWorkspace, { body: payload });
}

export function setWorkspacePromptEnabled(payload: PromptDetailPayload & { workspaceCode: string; enabled: boolean; expectedVersion: number }) {
  return requestApi<PromptRecord>(promptApiEndpoints.setEnabledWorkspace, { body: payload });
}

export function setPlatformPromptEnabled(payload: PromptDetailPayload & { enabled: boolean; expectedVersion: number }) {
  return requestApi<PromptRecord>(promptApiEndpoints.setEnabledPlatform, { body: payload });
}

export function setWorkspacePromptAllowInheritance(payload: PromptDetailPayload & { workspaceCode: string; allowInheritance: boolean; expectedVersion: number }) {
  return requestApi<PromptRecord>(promptApiEndpoints.setAllowInheritanceWorkspace, { body: payload });
}

export function analyzePromptTemplateApi(promptBody: string, signal?: AbortSignal) {
  return requestApi<PromptAnalyzeResult>(promptApiEndpoints.analyzeTemplate, { body: { promptBody }, signal });
}

export function exportWorkspacePrompts(payload: PromptPageQuery & { workspaceCode: string }) {
  return requestApi<PromptExportResult>(promptApiEndpoints.workspaceExport, { body: payload });
}
