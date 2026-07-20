export type ResourceScopeCode = "PLATFORM" | "WORKSPACE";
export type ResourceSourceCode = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";
export type ResourceLifecycleCode = "ACTIVE" | "DRAFT" | "HISTORICAL";
export type ResourceValidationCode = "VALID" | "INVALID";
export type ResourceDefinitionStatusCode = "ENABLED" | "DISABLED";

export interface ResourceApiPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}

/**
 * Resolved read-only limits for a definition's binaries. The raw
 * `binarySchema` remains the editable source configuration and may omit a
 * field that the backend supplies from its service-level defaults.
 */
export interface ResourceDefinitionEffectiveBinaryPolicyApiRecord {
  minCount: number;
  maxCount: number;
  maxSizeBytes: number;
  maxNameLength: number;
  namePattern: string | null;
}

export interface ResourceDefinitionVersionApiRecord {
  scopeKind: ResourceScopeCode;
  scopeCode: string;
  sourceKind: ResourceSourceCode;
  builtIn: boolean;
  definitionCode: string;
  definitionVersion: string;
  name: string;
  description: string;
  definitionStatus: ResourceDefinitionStatusCode;
  allowInheritance: boolean;
  lifecycleStatus: ResourceLifecycleCode;
  validationStatus: ResourceValidationCode;
  draftRevision: string | null;
  attributesSchema: Record<string, unknown> | null;
  binarySchema: Record<string, unknown> | null;
  /** Optional while a frontend is compatible with a pre-policy backend. */
  effectiveBinaryPolicy?: ResourceDefinitionEffectiveBinaryPolicyApiRecord | null;
  binaryStorageProfile: string | null;
  createdAt: number;
  updatedAt: number;
  publishedAt: number | null;
}

export interface ResourceBinaryApiRecord {
  binaryName: string;
  byteSize: number;
}

export interface ResourceVersionApiRecord {
  scopeKind: ResourceScopeCode;
  scopeCode: string;
  sourceKind: ResourceSourceCode;
  resourceCode: string;
  resourceVersion: string;
  definitionScopeKind: ResourceScopeCode;
  definitionScopeCode: string;
  definitionCode: string;
  definitionVersion: string;
  definitionName: string;
  definitionStatus: ResourceDefinitionStatusCode;
  name: string;
  description: string;
  attributes: Record<string, unknown> | null;
  attributesSchema: Record<string, unknown> | null;
  binarySchema: Record<string, unknown> | null;
  binaryStorageProfile: string | null;
  allowInheritance: boolean;
  lifecycleStatus: ResourceLifecycleCode;
  validationStatus: ResourceValidationCode;
  draftRevision: string | null;
  binaries: ResourceBinaryApiRecord[] | null;
  createdAt: number;
  updatedAt: number;
  publishedAt: number | null;
}

export interface ResourceValidationIssueApiRecord {
  path: string;
  message: string;
}

export interface ResourceDefinitionDraftActionApiResult {
  scopeKind: ResourceScopeCode;
  scopeCode: string;
  definitionCode: string;
  definitionVersion: string;
  draftRevision: string;
  lifecycleStatus: ResourceLifecycleCode;
  validationStatus: ResourceValidationCode;
  validationErrors: ResourceValidationIssueApiRecord[];
  discarded: boolean | null;
}

export interface ResourceDefinitionActivationApiResult {
  definitionCode: string;
  definitionVersion: string;
  lifecycleStatus: ResourceLifecycleCode;
  activationResult: "COMMITTED" | "DRAFT_PUBLISHED" | "HISTORICAL_REACTIVATED" | "ALREADY_ACTIVE";
  previousActiveDefinitionVersion: string | null;
}

export interface ResourceDraftActionApiResult {
  draftResult: string;
  resourceCode: string;
  resourceVersion: string;
  definitionScopeKind: ResourceScopeCode;
  definitionScopeCode: string;
  definitionCode: string;
  definitionVersion: string;
  draftRevision: string;
  validationStatus: ResourceValidationCode;
  validationErrors: ResourceValidationIssueApiRecord[];
  discarded: boolean | null;
  resourceRemoved: boolean | null;
}

export interface ResourceActivationApiResult {
  resourceCode: string;
  resourceVersion: string;
  definitionScopeKind: ResourceScopeCode;
  definitionScopeCode: string;
  definitionCode: string;
  definitionVersion: string;
  lifecycleStatus: ResourceLifecycleCode;
  activationResult: "COMMITTED" | "DRAFT_PUBLISHED" | "HISTORICAL_REACTIVATED" | "ALREADY_ACTIVE";
  previousActiveResourceVersion: string | null;
}

export interface ResourceBinaryActionApiResult {
  action: "UPLOADED" | "REMOVED";
  resourceCode: string;
  resourceVersion: string | null;
  binaryName: string;
  byteSize: number | null;
  draftRevision: string | null;
  validationStatus: ResourceValidationCode | null;
  validationErrors: ResourceValidationIssueApiRecord[];
}

export interface ResourceDefinitionPageApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  search: string | null;
  definitionCode: string | null;
  definitionStatus: ResourceDefinitionStatusCode | null;
  lifecycleStatus: ResourceLifecycleCode | null;
  validationStatus: ResourceValidationCode | null;
  sourceKind: ResourceSourceCode | null;
  current: number;
  size: number;
}

export interface ResourceDefinitionGetApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  definitionVersion: string;
}

export interface ResourceDefinitionCreateDraftApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  sourceDefinitionVersion: string | null;
  name: string;
  description: string | null;
  definitionStatus: ResourceDefinitionStatusCode | null;
  allowInheritance: boolean | null;
  attributesSchema: Record<string, unknown> | null;
  binarySchema: Record<string, unknown> | null;
  binaryStorageProfile: string | null;
}

export interface ResourceDefinitionUpdateDraftApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  definitionVersion: string;
  expectedDraftRevision: string;
  name?: string | null;
  description?: string | null;
  definitionStatus?: ResourceDefinitionStatusCode | null;
  allowInheritance?: boolean | null;
  attributesSchema?: Record<string, unknown> | null;
  binarySchema?: Record<string, unknown> | null;
  binaryStorageProfile?: string | null;
}

export interface ResourceDefinitionDiscardDraftApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  definitionVersion: string;
  expectedDraftRevision: string;
}

export interface ResourceDefinitionPublishDraftApiRequest extends ResourceDefinitionDiscardDraftApiRequest {}

export interface ResourceDefinitionActivateVersionApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  definitionVersion: string;
  expectedActiveDefinitionVersion: string | null;
}

export interface ResourceDefinitionSetEnabledApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  enabled: boolean;
  expectedActiveDefinitionVersion: string;
}

export interface ResourceDefinitionDeleteApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  definitionCode: string;
  expectedActiveDefinitionVersion: string;
}

export interface ResourceDefinitionDeleteApiResult {
  definitionCode: string;
  deleted: boolean;
}

export interface ResourcePageApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  search: string | null;
  resourceCode: string | null;
  definitionCode: string | null;
  definitionStatus: ResourceDefinitionStatusCode | null;
  lifecycleStatus: ResourceLifecycleCode | null;
  validationStatus: ResourceValidationCode | null;
  sourceKind: ResourceSourceCode | null;
  current: number;
  size: number;
}

export interface ResourceGetApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
}

export interface ResourceCreateDraftApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  sourceResourceVersion: string | null;
  definitionCode: string | null;
  name: string;
  description: string | null;
  attributes: Record<string, unknown> | null;
  allowInheritance: boolean | null;
}

export interface ResourceUpdateDraftApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
  expectedDraftRevision: string;
  name?: string | null;
  description?: string | null;
  attributes?: Record<string, unknown> | null;
  allowInheritance?: boolean | null;
}

export interface ResourceDiscardDraftApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
  expectedDraftRevision: string;
}

export interface ResourcePublishDraftApiRequest extends ResourceDiscardDraftApiRequest {}

export interface ResourceActivateVersionApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
  expectedActiveResourceVersion: string | null;
}

/**
 * A browser-selected file associated with its resource-level binary name.
 * The client transmits that name as the multipart filename; the file's local
 * name is intentionally not part of the Resource command payload.
 */
export interface ResourceCommitBinaryApiInput {
  binaryName: string;
  file: File;
}

/**
 * Shared browser command shape for the direct ACTIVE create and update paths.
 * `binaryFiles` is transport-only and is excluded from the JSON `command`
 * multipart field by the API client.
 */
export interface ResourceCommitApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  definitionCode: string | null;
  expectedActiveResourceVersion: string | null;
  name: string;
  description: string;
  attributes: Record<string, unknown>;
  allowInheritance: boolean;
  removedBinaryNames: string[];
  binaryFiles: ResourceCommitBinaryApiInput[];
}

export interface ResourceDeleteApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  expectedActiveResourceVersion: string | null;
  expectedDraftResourceVersion: string | null;
  expectedDraftRevision: string | null;
}

export interface ResourceDeleteApiResult {
  resourceCode: string;
  deleted: boolean;
}

export interface ResourceBinaryUploadApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
  expectedDraftRevision: string;
  binaryName: string;
  file: File;
}

export interface ResourceBinaryRemoveApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
  expectedDraftRevision: string;
  binaryName: string;
}

export interface ResourceBinaryDownloadApiRequest {
  scopeKind: ResourceScopeCode;
  workspaceCode: string | null;
  resourceCode: string;
  resourceVersion: string;
  binaryName: string;
}
