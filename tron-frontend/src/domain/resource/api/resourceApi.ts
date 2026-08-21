import {
  requestApi,
  requestApiBlob,
  type ApiBlobResponse,
} from "@/shared/api/httpClient";

import type {
  ResourceApiPage,
  ResourceApiRecord,
  ResourceBinaryDownloadApiRequest,
  ResourceBinaryUploadApiRequest,
  ResourceBinaryUploadApiResult,
  ResourceCommitApiResult,
  ResourceCreateApiRequest,
  ResourceDeleteApiRequest,
  ResourceDeleteApiResult,
  ResourceGetApiRequest,
  ResourcePageApiRequest,
  ResourceUpdateApiRequest,
} from "./resourceApiContract";
import { toResourceKindCode } from "./resourceApiContract";

export const resourceApiEndpoints = {
  resourceCreate: "POST /api/v1/resources/create",
  resourceDelete: "POST /api/v1/resources/delete",
  resourceGet: "POST /api/v1/resources/get",
  resourcePage: "POST /api/v1/resources/page",
  resourceUpdate: "POST /api/v1/resources/update",
  downloadBinary: "POST /api/v1/resources/download-binary",
  uploadBinary: "POST /api/v1/resources/upload-binary",
} as const;

export function pageResources(request: ResourcePageApiRequest) {
  return requestApi<ResourceApiPage<ResourceApiRecord>>(resourceApiEndpoints.resourcePage, {
    body: request,
  });
}

export function getResource(request: ResourceGetApiRequest) {
  return requestApi<ResourceApiRecord>(resourceApiEndpoints.resourceGet, {
    body: request,
  });
}

export function createResource(request: ResourceCreateApiRequest) {
  const { file, ...command } = request;
  const body = new FormData();
  body.set("command", JSON.stringify({ ...command, kind: toResourceKindCode(command.kind) }));
  body.set("binary", file, file.name);
  return requestApi<ResourceCommitApiResult>(resourceApiEndpoints.resourceCreate, { body });
}

export function updateResource(request: ResourceUpdateApiRequest) {
  return requestApi<ResourceCommitApiResult>(resourceApiEndpoints.resourceUpdate, {
    body: request,
  });
}

export function deleteResource(request: ResourceDeleteApiRequest) {
  return requestApi<ResourceDeleteApiResult>(resourceApiEndpoints.resourceDelete, {
    body: request,
  });
}

export function uploadResourceBinary(request: ResourceBinaryUploadApiRequest) {
  const body = new FormData();
  body.set("workspaceCode", request.workspaceCode);
  body.set("resourceCode", request.resourceCode);
  body.set("file", request.file);
  return requestApi<ResourceBinaryUploadApiResult>(resourceApiEndpoints.uploadBinary, { body });
}

export function downloadResourceBinary(
  request: ResourceBinaryDownloadApiRequest,
  signal?: AbortSignal,
): Promise<ApiBlobResponse> {
  return requestApiBlob(resourceApiEndpoints.downloadBinary, { body: request, signal });
}
