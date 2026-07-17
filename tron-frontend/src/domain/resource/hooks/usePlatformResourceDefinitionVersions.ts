import { useMemo } from "react";

import {
  listPlatformResourceDefinitionVersionDetails,
  listPlatformResourceDefinitionVersions,
} from "@/domain/resource/mock";

export function usePlatformResourceDefinitionVersions() {
  return useMemo(() => listPlatformResourceDefinitionVersions(), []);
}

export function usePlatformResourceDefinitionVersionDetails(definitionCode: string) {
  return useMemo(() => (
    listPlatformResourceDefinitionVersionDetails()
      .filter((record) => record.definitionCode === definitionCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt))
  ), [definitionCode]);
}

export function useActivePlatformResourceDefinitions() {
  return useMemo(() => (
    listPlatformResourceDefinitionVersionDetails()
      .filter((record) => record.lifecycleStatus === "ACTIVE" && record.enabled)
      .sort((left, right) => left.name.localeCompare(right.name))
  ), []);
}
