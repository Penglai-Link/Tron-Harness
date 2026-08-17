import { requestApi } from "@/shared/api/httpClient";

import type {
  ModelCapability,
  ModelProtocol,
  ModelRecord,
  ReasoningEffort,
  ModelSelectOption,
  ModelType,
  WorkspaceModelCatalog,
} from "@/domain/model/model";
import {
  normalizeModelCapabilities,
  normalizeReasoningEfforts,
} from "@/domain/model/constants/modelPageOptions";

export const modelApiEndpoints = {
  listPlatformModels: "GET /api/v1/admin/platform/models",
  createPlatformModel: "POST /api/v1/admin/platform/models",
  verifyPlatformModelDraft: "POST /api/v1/admin/platform/models/verify",
  getPlatformModel: "GET /api/v1/admin/platform/models/{modelCode}",
  updatePlatformModel: "PATCH /api/v1/admin/platform/models/{modelCode}",
  verifyPlatformModel: "POST /api/v1/admin/platform/models/{modelCode}/verify",
  duplicatePlatformModel: "POST /api/v1/admin/platform/models/{modelCode}/duplicate",
  enablePlatformModel: "POST /api/v1/admin/platform/models/{modelCode}/enable",
  disablePlatformModel: "POST /api/v1/admin/platform/models/{modelCode}/disable",
  setPlatformDefault: "POST /api/v1/admin/platform/models/{modelCode}/set-default",
  deletePlatformModel: "DELETE /api/v1/admin/platform/models/{modelCode}",
  listWorkspaceModels: "GET /api/v1/models/workspaces/{workspaceCode}",
  listWorkspaceSelectOptions: "GET /api/v1/models/workspaces/{workspaceCode}/select-options",
  createWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}",
  verifyWorkspaceModelDraft: "POST /api/v1/models/workspaces/{workspaceCode}/verify",
  getWorkspaceModel: "GET /api/v1/models/workspaces/{workspaceCode}/{modelCode}",
  updateWorkspaceModel: "PATCH /api/v1/models/workspaces/{workspaceCode}/{modelCode}",
  verifyWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/verify",
  duplicateWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/duplicate",
  enableWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/enable",
  disableWorkspaceModel: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/disable",
  setWorkspaceDefault: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/set-default",
  setWorkspaceAllowInheritance: "POST /api/v1/models/workspaces/{workspaceCode}/{modelCode}/allow-inheritance",
  deleteWorkspaceModel: "DELETE /api/v1/models/workspaces/{workspaceCode}/{modelCode}",
} as const;

export interface ModelMutationPayload {
  displayName: string;
  vendor: string;
  protocol: ModelProtocol;
  modelType: ModelType;
  modelCapabilities: ModelCapability[];
  baseUrl: string;
  apiPath?: string | null;
  apiKeyRef: string;
  modelName: string;
  enabled: boolean;
  description?: string | null;
  thinkingEnabled: boolean;
  supportedReasoningEfforts: ReasoningEffort[];
  defaultReasoningEffort?: ReasoningEffort | null;
  structuredOutputEnabled: boolean;
  responsesStatefulEnabled: boolean;
  temperature?: number | null;
  maxTokens?: number | null;
  contextWindowTokens?: number | null;
  topP?: number | null;
  maxRetries?: number | null;
  allowInheritance?: boolean;
}

export interface WorkspaceDefaultMutationResult {
  workspaceCode: string;
  modelType: ModelType;
  modelCode: string;
}

export interface PlatformDefaultMutationResult {
  modelType: ModelType;
  modelCode: string;
}

export interface ModelMutationResult {
  workspaceCode?: string | null;
  modelCode: string;
}

const MASKED_API_KEY_VALUE = "*****";

function sanitizeModelRecord(record: ModelRecord): ModelRecord {
  const supportedReasoningEfforts = normalizeReasoningEfforts(record.supportedReasoningEfforts);
  return {
    ...record,
    modelCapabilities: normalizeModelCapabilities(record.modelCapabilities, record.modelType),
    supportedReasoningEfforts,
    defaultReasoningEffort: record.defaultReasoningEffort
      && supportedReasoningEfforts.includes(record.defaultReasoningEffort)
      ? record.defaultReasoningEffort
      : null,
    apiKeyRef: record.apiKeyRef.trim() ? MASKED_API_KEY_VALUE : "",
  };
}

function sanitizeWorkspaceModelCatalog(catalog: WorkspaceModelCatalog): WorkspaceModelCatalog {
  return {
    ...catalog,
    records: catalog.records.map((record) => sanitizeModelRecord(record)),
  };
}

export function fetchPlatformModels(signal?: AbortSignal) {
  return requestApi<ModelRecord[]>(modelApiEndpoints.listPlatformModels, { signal })
    .then((records) => records.map((record) => sanitizeModelRecord(record)));
}

export function fetchPlatformModel(modelCode: string, signal?: AbortSignal) {
  return requestApi<ModelRecord>(modelApiEndpoints.getPlatformModel, {
    pathParams: { modelCode },
    signal,
  }).then((record) => sanitizeModelRecord(record));
}

export function createPlatformModel(payload: ModelMutationPayload) {
  return requestApi<ModelRecord>(modelApiEndpoints.createPlatformModel, { body: payload })
    .then((record) => sanitizeModelRecord(record));
}

export function verifyPlatformModelDraft(payload: ModelMutationPayload) {
  return requestApi<ModelRecord>(modelApiEndpoints.verifyPlatformModelDraft, { body: payload })
    .then((record) => sanitizeModelRecord(record));
}

export function updatePlatformModel(modelCode: string, payload: ModelMutationPayload) {
  return requestApi<ModelRecord>(modelApiEndpoints.updatePlatformModel, {
    body: payload,
    pathParams: { modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function verifyPlatformModel(modelCode: string, payload?: ModelMutationPayload) {
  return requestApi<ModelRecord>(modelApiEndpoints.verifyPlatformModel, {
    body: payload,
    pathParams: { modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function duplicatePlatformModel(modelCode: string) {
  return requestApi<ModelRecord>(modelApiEndpoints.duplicatePlatformModel, {
    pathParams: { modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function enablePlatformModel(modelCode: string) {
  return requestApi<ModelRecord>(modelApiEndpoints.enablePlatformModel, {
    pathParams: { modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function disablePlatformModel(modelCode: string) {
  return requestApi<ModelRecord>(modelApiEndpoints.disablePlatformModel, {
    pathParams: { modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function setPlatformDefaultModel(modelCode: string) {
  return requestApi<PlatformDefaultMutationResult>(modelApiEndpoints.setPlatformDefault, {
    pathParams: { modelCode },
  });
}

export function deletePlatformModel(modelCode: string) {
  return requestApi<ModelMutationResult>(modelApiEndpoints.deletePlatformModel, {
    pathParams: { modelCode },
  });
}

export function fetchWorkspaceModelCatalog(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<WorkspaceModelCatalog>(modelApiEndpoints.listWorkspaceModels, {
    pathParams: { workspaceCode },
    signal,
  }).then((catalog) => sanitizeWorkspaceModelCatalog(catalog));
}

export function fetchWorkspaceSelectOptions(workspaceCode: string, signal?: AbortSignal) {
  return requestApi<ModelSelectOption[]>(modelApiEndpoints.listWorkspaceSelectOptions, {
    pathParams: { workspaceCode },
    signal,
  });
}

export function createWorkspaceModel(
  workspaceCode: string,
  payload: ModelMutationPayload,
) {
  return requestApi<ModelRecord>(modelApiEndpoints.createWorkspaceModel, {
    body: payload,
    pathParams: { workspaceCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function verifyWorkspaceModelDraft(
  workspaceCode: string,
  payload: ModelMutationPayload,
) {
  return requestApi<ModelRecord>(modelApiEndpoints.verifyWorkspaceModelDraft, {
    body: payload,
    pathParams: { workspaceCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function updateWorkspaceModel(
  workspaceCode: string,
  modelCode: string,
  payload: ModelMutationPayload,
) {
  return requestApi<ModelRecord>(modelApiEndpoints.updateWorkspaceModel, {
    body: payload,
    pathParams: { workspaceCode, modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function verifyWorkspaceModel(
  workspaceCode: string,
  modelCode: string,
  payload?: ModelMutationPayload,
) {
  return requestApi<ModelRecord>(modelApiEndpoints.verifyWorkspaceModel, {
    body: payload,
    pathParams: { workspaceCode, modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function duplicateWorkspaceModel(workspaceCode: string, modelCode: string) {
  return requestApi<ModelRecord>(modelApiEndpoints.duplicateWorkspaceModel, {
    pathParams: { workspaceCode, modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function enableWorkspaceModel(workspaceCode: string, modelCode: string) {
  return requestApi<ModelRecord>(modelApiEndpoints.enableWorkspaceModel, {
    pathParams: { workspaceCode, modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function disableWorkspaceModel(workspaceCode: string, modelCode: string) {
  return requestApi<ModelRecord>(modelApiEndpoints.disableWorkspaceModel, {
    pathParams: { workspaceCode, modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function setWorkspaceDefaultModel(workspaceCode: string, modelCode: string) {
  return requestApi<WorkspaceDefaultMutationResult>(modelApiEndpoints.setWorkspaceDefault, {
    pathParams: { workspaceCode, modelCode },
  });
}

export function setWorkspaceAllowInheritance(
  workspaceCode: string,
  modelCode: string,
  allowInheritance: boolean,
) {
  return requestApi<ModelRecord>(modelApiEndpoints.setWorkspaceAllowInheritance, {
    body: { allowInheritance },
    pathParams: { workspaceCode, modelCode },
  }).then((record) => sanitizeModelRecord(record));
}

export function deleteWorkspaceModel(workspaceCode: string, modelCode: string) {
  return requestApi<ModelMutationResult>(modelApiEndpoints.deleteWorkspaceModel, {
    pathParams: { workspaceCode, modelCode },
  });
}
