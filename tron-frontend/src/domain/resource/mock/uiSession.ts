import type {
  ResourceDefinitionVersionDetail,
  ResourceVersionDetail,
  ResourceVersionRecord,
} from "@/domain/resource/model";

const sessionPrefix = "tron.resource-ui";

function readSessionValue<T>(key: string): T | null {
  if (typeof window === "undefined") return null;
  try {
    const value = window.sessionStorage.getItem(key);
    return value ? JSON.parse(value) as T : null;
  } catch {
    return null;
  }
}

function writeSessionValue<T>(key: string, value: T) {
  if (typeof window === "undefined") return;
  window.sessionStorage.setItem(key, JSON.stringify(value));
}

function versionKey(kind: "definition" | "resource", scopeCode: string, businessCode: string) {
  return `${sessionPrefix}.${kind}.${scopeCode || "unknown"}.${businessCode || "new"}`;
}

function resourcePageKey(scopeCode: string) {
  return `${sessionPrefix}.resource-page.${scopeCode || "unknown"}`;
}

export function readResourceDefinitionVersionsFromUiSession(scopeCode: string, definitionCode: string) {
  return readSessionValue<ResourceDefinitionVersionDetail[]>(versionKey("definition", scopeCode, definitionCode));
}

export function writeResourceDefinitionVersionsToUiSession(
  scopeCode: string,
  definitionCode: string,
  versions: ResourceDefinitionVersionDetail[],
) {
  writeSessionValue(versionKey("definition", scopeCode, definitionCode), versions);
}

export function readResourceVersionsFromUiSession(scopeCode: string, resourceCode: string) {
  return readSessionValue<ResourceVersionDetail[]>(versionKey("resource", scopeCode, resourceCode));
}

export function writeResourceVersionsToUiSession(
  scopeCode: string,
  resourceCode: string,
  versions: ResourceVersionDetail[],
) {
  writeSessionValue(versionKey("resource", scopeCode, resourceCode), versions);
}

/**
 * The Mock list has a small session-only overlay for actions that create a
 * resource directly from the list. It avoids pretending that an API request
 * happened while still letting the resulting version appear after refresh.
 */
export function readMockCreatedResourceRecords(scopeCode: string) {
  return readSessionValue<ResourceVersionRecord[]>(resourcePageKey(scopeCode)) ?? [];
}

export function writeMockCreatedResourceRecords(scopeCode: string, records: ResourceVersionRecord[]) {
  writeSessionValue(resourcePageKey(scopeCode), records);
}
