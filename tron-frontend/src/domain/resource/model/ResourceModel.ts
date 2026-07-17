import type {
  ResourceDefinitionAttributesSchema,
  ResourceDefinitionBinarySchema,
} from "./ResourceDefinitionModel";

export type ResourceLifecycleStatus = "ACTIVE" | "DRAFT" | "HISTORICAL";

export type ResourceValidationStatus = "VALID" | "INVALID";

export type ResourceScopeKind = "PLATFORM" | "WORKSPACE";

export type ResourceSourceKind = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";

export interface ResourceVersionRecord {
  definitionCode: string;
  definitionName: string;
  definitionVersion: string;
  description: string;
  draftRevision: number | null;
  lifecycleStatus: ResourceLifecycleStatus;
  name: string;
  ownerWorkspaceCode: string | null;
  ownerWorkspaceName: string | null;
  resourceCode: string;
  resourceVersion: string;
  scopeKind: ResourceScopeKind;
  sourceKind: ResourceSourceKind;
  updatedAt: string;
  validationIssueCount: number;
  validationStatus: ResourceValidationStatus;
}

export interface ResourceBinaryMapping {
  binaryName: string;
}

export interface ResourceVersionDetail extends ResourceVersionRecord {
  allowInheritance: boolean;
  attributes: Record<string, unknown>;
  attributesSchema: ResourceDefinitionAttributesSchema;
  binaries: ResourceBinaryMapping[];
  binarySchema: ResourceDefinitionBinarySchema;
  createdAt: string;
  publishedAt: string | null;
}

export type ResourceLifecycleFilter = ResourceLifecycleStatus | "ALL";
export type ResourceValidationFilter = ResourceValidationStatus | "ALL";
export type ResourceContextFilter = ResourceScopeKind | ResourceSourceKind | "ALL";
