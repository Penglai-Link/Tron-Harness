import {
  requestApi,
  requestApiBlob,
  type ApiBlobResponse,
} from "@/shared/api/httpClient";

import type {
  ResourceActivateVersionApiRequest,
  ResourceActivationApiResult,
  ResourceBinaryActionApiResult,
  ResourceBinaryDownloadApiRequest,
  ResourceBinaryRemoveApiRequest,
  ResourceBinaryUploadApiRequest,
  ResourceCreateDraftApiRequest,
  ResourceCommitApiRequest,
  ResourceDeleteApiRequest,
  ResourceDeleteApiResult,
  ResourceDefinitionActivateVersionApiRequest,
  ResourceDefinitionActivationApiResult,
  ResourceDefinitionCreateDraftApiRequest,
  ResourceDefinitionDiscardDraftApiRequest,
  ResourceDefinitionDeleteApiRequest,
  ResourceDefinitionDeleteApiResult,
  ResourceDefinitionDraftActionApiResult,
  ResourceDefinitionGetApiRequest,
  ResourceDefinitionPageApiRequest,
  ResourceDefinitionPublishDraftApiRequest,
  ResourceDefinitionSetEnabledApiRequest,
  ResourceDefinitionUpdateDraftApiRequest,
  ResourceDefinitionVersionApiRecord,
  ResourceDiscardDraftApiRequest,
  ResourceDraftActionApiResult,
  ResourceGetApiRequest,
  ResourcePageApiRequest,
  ResourcePublishDraftApiRequest,
  ResourceUpdateDraftApiRequest,
  ResourceVersionApiRecord,
  ResourceApiPage,
} from "./resourceApiContract";

export const resourceApiEndpoints = {
  definitionActivateVersion: "POST /api/v1/resource-definitions/activate-version",
  definitionCreateDraft: "POST /api/v1/resource-definitions/create-draft",
  definitionDelete: "POST /api/v1/resource-definitions/delete",
  definitionDiscardDraft: "POST /api/v1/resource-definitions/discard-draft",
  definitionGet: "POST /api/v1/resource-definitions/get",
  definitionPage: "POST /api/v1/resource-definitions/page",
  definitionPublishDraft: "POST /api/v1/resource-definitions/publish-draft",
  definitionSetEnabled: "POST /api/v1/resource-definitions/set-enabled",
  definitionUpdateDraft: "POST /api/v1/resource-definitions/update-draft",
  downloadBinary: "POST /api/v1/resources/download-binary",
  removeBinary: "POST /api/v1/resources/remove-binary",
  resourceActivateVersion: "POST /api/v1/resources/activate-version",
  resourceCreate: "POST /api/v1/resources/create",
  resourceCreateDraft: "POST /api/v1/resources/create-draft",
  resourceDelete: "POST /api/v1/resources/delete",
  resourceDiscardDraft: "POST /api/v1/resources/discard-draft",
  resourceGet: "POST /api/v1/resources/get",
  resourcePage: "POST /api/v1/resources/page",
  resourcePublishDraft: "POST /api/v1/resources/publish-draft",
  resourceUpdateDraft: "POST /api/v1/resources/update-draft",
  resourceUpdate: "POST /api/v1/resources/update",
  uploadBinary: "POST /api/v1/resources/upload-binary",
} as const;

export function pageResourceDefinitions(request: ResourceDefinitionPageApiRequest) {
  return requestApi<ResourceApiPage<ResourceDefinitionVersionApiRecord>>(
    resourceApiEndpoints.definitionPage,
    { body: request },
  );
}

export function getResourceDefinition(request: ResourceDefinitionGetApiRequest) {
  return requestApi<ResourceDefinitionVersionApiRecord>(resourceApiEndpoints.definitionGet, {
    body: request,
  });
}

export function createResourceDefinitionDraft(request: ResourceDefinitionCreateDraftApiRequest) {
  return requestApi<ResourceDefinitionDraftActionApiResult>(
    resourceApiEndpoints.definitionCreateDraft,
    { body: request },
  );
}

export function updateResourceDefinitionDraft(request: ResourceDefinitionUpdateDraftApiRequest) {
  return requestApi<ResourceDefinitionDraftActionApiResult>(
    resourceApiEndpoints.definitionUpdateDraft,
    { body: request },
  );
}

export function discardResourceDefinitionDraft(request: ResourceDefinitionDiscardDraftApiRequest) {
  return requestApi<ResourceDefinitionDraftActionApiResult>(
    resourceApiEndpoints.definitionDiscardDraft,
    { body: request },
  );
}

export function publishResourceDefinitionDraft(request: ResourceDefinitionPublishDraftApiRequest) {
  return requestApi<ResourceDefinitionActivationApiResult>(
    resourceApiEndpoints.definitionPublishDraft,
    { body: request },
  );
}

export function activateResourceDefinitionVersion(request: ResourceDefinitionActivateVersionApiRequest) {
  return requestApi<ResourceDefinitionActivationApiResult>(
    resourceApiEndpoints.definitionActivateVersion,
    { body: request },
  );
}

export function setResourceDefinitionEnabled(request: ResourceDefinitionSetEnabledApiRequest) {
  return requestApi<ResourceDefinitionActivationApiResult>(
    resourceApiEndpoints.definitionSetEnabled,
    { body: request },
  );
}

export function deleteResourceDefinition(request: ResourceDefinitionDeleteApiRequest) {
  return requestApi<ResourceDefinitionDeleteApiResult>(resourceApiEndpoints.definitionDelete, {
    body: request,
  });
}

export function pageResources(request: ResourcePageApiRequest) {
  return requestApi<ResourceApiPage<ResourceVersionApiRecord>>(resourceApiEndpoints.resourcePage, {
    body: request,
  });
}

export function getResource(request: ResourceGetApiRequest) {
  return requestApi<ResourceVersionApiRecord>(resourceApiEndpoints.resourceGet, {
    body: request,
  });
}

export function createResourceDraft(request: ResourceCreateDraftApiRequest) {
  return requestApi<ResourceDraftActionApiResult>(resourceApiEndpoints.resourceCreateDraft, {
    body: request,
  });
}

export function updateResourceDraft(request: ResourceUpdateDraftApiRequest) {
  return requestApi<ResourceDraftActionApiResult>(resourceApiEndpoints.resourceUpdateDraft, {
    body: request,
  });
}

export function discardResourceDraft(request: ResourceDiscardDraftApiRequest) {
  return requestApi<ResourceDraftActionApiResult>(resourceApiEndpoints.resourceDiscardDraft, {
    body: request,
  });
}

export function publishResourceDraft(request: ResourcePublishDraftApiRequest) {
  return requestApi<ResourceActivationApiResult>(resourceApiEndpoints.resourcePublishDraft, {
    body: request,
  });
}

export function activateResourceVersion(request: ResourceActivateVersionApiRequest) {
  return requestApi<ResourceActivationApiResult>(resourceApiEndpoints.resourceActivateVersion, {
    body: request,
  });
}

function resourceCommitFormData(request: ResourceCommitApiRequest) {
  const { binaryFiles, ...command } = request;
  const body = new FormData();
  body.set("command", JSON.stringify(command));
  binaryFiles.forEach(({ binaryName, file }) => {
    body.append("binary", file, binaryName);
  });
  return body;
}

/**
 * Creates a fully validated resource and immediately activates its first
 * version. Files are committed atomically with the structured attributes.
 */
export function createResource(request: ResourceCommitApiRequest) {
  return requestApi<ResourceActivationApiResult>(resourceApiEndpoints.resourceCreate, {
    body: resourceCommitFormData(request),
  });
}

/**
 * Creates and immediately activates the next immutable Resource version.
 */
export function updateResource(request: ResourceCommitApiRequest) {
  return requestApi<ResourceActivationApiResult>(resourceApiEndpoints.resourceUpdate, {
    body: resourceCommitFormData(request),
  });
}

export function deleteResource(request: ResourceDeleteApiRequest) {
  return requestApi<ResourceDeleteApiResult>(resourceApiEndpoints.resourceDelete, {
    body: request,
  });
}

export function uploadResourceBinary(request: ResourceBinaryUploadApiRequest) {
  const body = new FormData();
  body.set("scopeKind", request.scopeKind);
  if (request.workspaceCode !== null) body.set("workspaceCode", request.workspaceCode);
  body.set("resourceCode", request.resourceCode);
  body.set("resourceVersion", request.resourceVersion);
  body.set("expectedDraftRevision", request.expectedDraftRevision);
  body.set("binaryName", request.binaryName);
  body.set("file", request.file);
  return requestApi<ResourceBinaryActionApiResult>(resourceApiEndpoints.uploadBinary, { body });
}

export function removeResourceBinary(request: ResourceBinaryRemoveApiRequest) {
  return requestApi<ResourceBinaryActionApiResult>(resourceApiEndpoints.removeBinary, {
    body: request,
  });
}

export function downloadResourceBinary(
  request: ResourceBinaryDownloadApiRequest,
): Promise<ApiBlobResponse> {
  return requestApiBlob(resourceApiEndpoints.downloadBinary, { body: request });
}
