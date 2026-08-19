import {
  fromResourceKindCode,
  fromResourceSourceCode,
  type ResourceApiRecord,
} from "@/domain/resource/api";
import type { ResourceRecord } from "@/domain/resource/model";

function toTimestamp(value: number) {
  const timestamp = new Date(value).toISOString();
  if (Number.isNaN(Date.parse(timestamp))) {
    throw new Error("Resource API returned an invalid timestamp");
  }
  return timestamp;
}

export function adaptResourceRecord(record: ResourceApiRecord): ResourceRecord {
  return {
    allowInheritance: record.allowInheritance,
    attributes: record.attributes,
    createdAt: toTimestamp(record.createdAt),
    description: record.description,
    file: record.file,
    kind: fromResourceKindCode(record.kind),
    name: record.name,
    resourceCode: record.resourceCode,
    sourceKind: fromResourceSourceCode(record.sourceKind),
    updatedAt: toTimestamp(record.updatedAt),
    workspaceCode: record.workspaceCode,
  };
}
