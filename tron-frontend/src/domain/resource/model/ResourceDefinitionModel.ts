export type ResourceDefinitionLifecycleStatus = "ACTIVE" | "DRAFT" | "HISTORICAL";

export type ResourceDefinitionValidationStatus = "VALID" | "INVALID";

export type ResourceDefinitionScopeKind = "PLATFORM" | "WORKSPACE";

export type ResourceDefinitionSourceKind = "PLATFORM" | "LOCAL" | "INHERITED" | "OVERRIDE";

export interface ResourceDefinitionVersionRecord {
  definitionCode: string;
  definitionVersion: string;
  description: string;
  draftRevision: number | null;
  enabled: boolean;
  lifecycleStatus: ResourceDefinitionLifecycleStatus;
  name: string;
  ownerWorkspaceCode: string | null;
  ownerWorkspaceName: string | null;
  preset: boolean;
  scopeKind: ResourceDefinitionScopeKind;
  sourceKind: ResourceDefinitionSourceKind;
  updatedAt: string;
  validationIssueCount: number;
  validationStatus: ResourceDefinitionValidationStatus;
}

export type ResourceDefinitionSchemaValueType = Exclude<JsonSchemaDraft7Type, "null">;

export type ResourceDefinitionAttributeProperty = JsonSchemaDraft7;

export interface ResourceDefinitionAttributesSchema extends JsonSchemaDraft7RootObject {}

export interface ResourceDefinitionBinarySchema {
  maxCount: number;
  maxNameLength: number;
  maxSizeBytes: number;
  minCount: number;
  namePattern: string;
  storageProfile: string;
}

export interface ResourceDefinitionVersionDetail extends ResourceDefinitionVersionRecord {
  allowInheritance: boolean;
  attributesSchema: ResourceDefinitionAttributesSchema;
  binarySchema: ResourceDefinitionBinarySchema;
  createdAt: string;
  publishedAt: string | null;
}

export type ResourceDefinitionLifecycleFilter = ResourceDefinitionLifecycleStatus | "ALL";
export type ResourceDefinitionEnabledFilter = "ALL" | "ENABLED" | "DISABLED";
export type ResourceDefinitionValidationFilter = ResourceDefinitionValidationStatus | "ALL";
import type {
  JsonSchemaDraft7,
  JsonSchemaDraft7RootObject,
  JsonSchemaDraft7Type,
} from "@/shared/ui/json-schema-draft7";
