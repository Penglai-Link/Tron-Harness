import type {
  ResourceDefinitionVersionApiRecord,
  ResourceVersionApiRecord,
} from "@/domain/resource/api";
import type {
  ResourceDefinitionAttributesSchema,
  ResourceDefinitionBinarySchema,
  ResourceDefinitionEffectiveBinaryPolicy,
  ResourceDefinitionVersionDetail,
  ResourceDefinitionVersionRecord,
  ResourceDefinitionValidationStatus,
} from "@/domain/resource/model/ResourceDefinitionModel";
import type {
  ResourceBinaryMapping,
  ResourceValidationStatus,
  ResourceVersionDetail,
  ResourceVersionRecord,
} from "@/domain/resource/model/ResourceModel";

const defaultAttributesSchema: ResourceDefinitionAttributesSchema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  additionalProperties: false,
  properties: {},
  required: [],
  type: "object",
};

function toTimestamp(value: number | null) {
  return value === null ? null : new Date(value).toISOString();
}

function toRequiredTimestamp(value: number) {
  const timestamp = toTimestamp(value);
  if (timestamp === null || Number.isNaN(Date.parse(timestamp))) {
    throw new Error("Resource API returned an invalid timestamp");
  }
  return timestamp;
}

function toDraftRevision(value: string | null) {
  if (value === null) return null;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error("Resource API returned an unsupported draft revision");
  }
  return parsed;
}

function asRecord(value: Record<string, unknown> | null) {
  return value ?? {};
}

function numberValue(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) && value >= 0
    ? Math.floor(value)
    : fallback;
}

function stringValue(value: unknown, fallback = "") {
  return typeof value === "string" ? value : fallback;
}

function nullableStringValue(value: unknown) {
  return typeof value === "string" ? value : null;
}

function mapBinarySchema(
  value: Record<string, unknown> | null,
  binaryStorageProfile: string | null,
): ResourceDefinitionBinarySchema {
  const schema = asRecord(value);
  return {
    maxCount: numberValue(schema.maxCount, 0),
    maxNameLength: numberValue(schema.maxNameLength, 255),
    maxSizeBytes: numberValue(schema.maxSizeBytes, 0),
    minCount: numberValue(schema.minCount, 0),
    namePattern: stringValue(schema.namePattern),
    storageProfile: binaryStorageProfile ?? "@default",
  };
}

function mapEffectiveBinaryPolicy(
  value: ResourceDefinitionVersionApiRecord["effectiveBinaryPolicy"],
): ResourceDefinitionEffectiveBinaryPolicy | null {
  if (!value || typeof value !== "object") return null;

  const minCount = numberValue(value.minCount, -1);
  const maxCount = numberValue(value.maxCount, -1);
  const maxSizeBytes = numberValue(value.maxSizeBytes, -1);
  const maxNameLength = numberValue(value.maxNameLength, -1);
  if (
    minCount < 0
    || maxCount < 0
    || minCount > maxCount
    || maxSizeBytes < 0
    || maxNameLength < 1
  ) {
    return null;
  }

  return {
    maxCount,
    maxNameLength,
    maxSizeBytes,
    minCount,
    namePattern: nullableStringValue(value.namePattern),
  };
}

function mapDefinitionRecord(
  record: ResourceDefinitionVersionApiRecord,
): ResourceDefinitionVersionRecord {
  return {
    definitionCode: record.definitionCode,
    definitionVersion: record.definitionVersion,
    description: record.description,
    draftRevision: toDraftRevision(record.draftRevision),
    enabled: record.definitionStatus === "ENABLED",
    lifecycleStatus: record.lifecycleStatus,
    name: record.name,
    ownerWorkspaceCode: record.scopeKind === "PLATFORM" ? null : record.scopeCode,
    ownerWorkspaceName: record.scopeKind === "PLATFORM" ? null : record.scopeCode,
    // The server owns this distinction: platform-scoped custom definitions
    // remain deletable, whereas built-ins are protected by domain rules.
    preset: record.builtIn,
    scopeKind: record.scopeKind,
    sourceKind: record.sourceKind,
    updatedAt: toRequiredTimestamp(record.updatedAt),
    validationIssueCount: 0,
    validationStatus: record.validationStatus as ResourceDefinitionValidationStatus,
  };
}

export function adaptResourceDefinitionVersionRecord(
  record: ResourceDefinitionVersionApiRecord,
): ResourceDefinitionVersionRecord {
  return mapDefinitionRecord(record);
}

export function adaptResourceDefinitionVersionDetail(
  record: ResourceDefinitionVersionApiRecord,
): ResourceDefinitionVersionDetail {
  return {
    ...mapDefinitionRecord(record),
    allowInheritance: record.allowInheritance,
    attributesSchema: asRecord(record.attributesSchema) as ResourceDefinitionAttributesSchema,
    binarySchema: mapBinarySchema(record.binarySchema, record.binaryStorageProfile),
    effectiveBinaryPolicy: mapEffectiveBinaryPolicy(record.effectiveBinaryPolicy),
    createdAt: toRequiredTimestamp(record.createdAt),
    publishedAt: toTimestamp(record.publishedAt),
  };
}

function mapResourceRecord(record: ResourceVersionApiRecord): ResourceVersionRecord {
  return {
    definitionCode: record.definitionCode,
    definitionName: record.definitionName,
    definitionVersion: record.definitionVersion,
    description: record.description,
    draftRevision: toDraftRevision(record.draftRevision),
    lifecycleStatus: record.lifecycleStatus,
    name: record.name,
    ownerWorkspaceCode: record.scopeKind === "PLATFORM" ? null : record.scopeCode,
    ownerWorkspaceName: record.scopeKind === "PLATFORM" ? null : record.scopeCode,
    resourceCode: record.resourceCode,
    resourceVersion: record.resourceVersion,
    scopeKind: record.scopeKind,
    sourceKind: record.sourceKind,
    updatedAt: toRequiredTimestamp(record.updatedAt),
    validationIssueCount: 0,
    validationStatus: record.validationStatus as ResourceValidationStatus,
  };
}

export function adaptResourceVersionRecord(record: ResourceVersionApiRecord): ResourceVersionRecord {
  return mapResourceRecord(record);
}

export function adaptResourceVersionDetail(record: ResourceVersionApiRecord): ResourceVersionDetail {
  return {
    ...mapResourceRecord(record),
    allowInheritance: record.allowInheritance,
    attributes: record.attributes ?? {},
    attributesSchema: (record.attributesSchema ?? defaultAttributesSchema) as ResourceDefinitionAttributesSchema,
    binaries: (record.binaries ?? []).map<ResourceBinaryMapping>((binary) => ({
      binaryName: binary.binaryName,
      byteSize: binary.byteSize,
    })),
    binarySchema: mapBinarySchema(record.binarySchema, record.binaryStorageProfile),
    createdAt: toRequiredTimestamp(record.createdAt),
    publishedAt: toTimestamp(record.publishedAt),
  };
}
