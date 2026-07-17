import type {
  ResourceDefinitionVersionDetail,
  ResourceVersionDetail,
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
