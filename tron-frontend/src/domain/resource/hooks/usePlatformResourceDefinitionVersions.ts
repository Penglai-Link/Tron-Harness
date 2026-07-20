import {
  listPlatformResourceDefinitionVersionDetails,
  listPlatformResourceDefinitionVersions,
} from "@/domain/resource/mock";

import {
  adaptResourceDefinitionVersionDetail,
  adaptResourceDefinitionVersionRecord,
} from "@/domain/resource/adapters";
import { getResourceDefinition } from "@/domain/resource/api";
import type {
  ResourceDefinitionVersionDetail,
  ResourceDefinitionVersionRecord,
} from "@/domain/resource/model";

import { pageAllResourceDefinitions } from "./resourceDefinitionPagination";
import { useResourceApiQuery } from "./useResourceApiQuery";

function loadPlatformDefinitions(definitionCode: string | null) {
  return pageAllResourceDefinitions({
    definitionCode,
    definitionStatus: null,
    lifecycleStatus: null,
    scopeKind: "PLATFORM",
    search: null,
    sourceKind: null,
    validationStatus: null,
    workspaceCode: null,
  });
}

export function usePlatformResourceDefinitionVersions() {
  const query = useResourceApiQuery<ResourceDefinitionVersionRecord[]>({
    api: async () => (await loadPlatformDefinitions(null))
      .map(adaptResourceDefinitionVersionRecord),
    deps: [],
    initialData: () => [],
    mock: () => listPlatformResourceDefinitionVersions(),
  });
  return query.data;
}

export function usePlatformResourceDefinitionVersionDetails(definitionCode: string) {
  const query = useResourceApiQuery<ResourceDefinitionVersionDetail[]>({
    api: async () => {
      const records = await loadPlatformDefinitions(definitionCode);
      return Promise.all(records.map(async (record) => (
        adaptResourceDefinitionVersionDetail(await getResourceDefinition({
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          scopeKind: "PLATFORM",
          workspaceCode: null,
        }))
      )));
    },
    deps: [definitionCode],
    enabled: Boolean(definitionCode),
    initialData: () => [],
    mock: () => listPlatformResourceDefinitionVersionDetails()
      .filter((record) => record.definitionCode === definitionCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt)),
  });
  return query.data;
}

export function useActivePlatformResourceDefinitions() {
  const query = useResourceApiQuery<ResourceDefinitionVersionDetail[]>({
    api: async () => {
      const records = await pageAllResourceDefinitions({
        definitionCode: null,
        definitionStatus: "ENABLED",
        lifecycleStatus: "ACTIVE",
        scopeKind: "PLATFORM",
        search: null,
        sourceKind: null,
        validationStatus: null,
        workspaceCode: null,
      });
      return Promise.all(records.map(async (record) => (
        adaptResourceDefinitionVersionDetail(await getResourceDefinition({
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          scopeKind: "PLATFORM",
          workspaceCode: null,
        }))
      )));
    },
    deps: [],
    initialData: () => [],
    mock: () => listPlatformResourceDefinitionVersionDetails()
      .filter((record) => record.lifecycleStatus === "ACTIVE" && record.enabled),
  });
  return query.data.slice().sort((left, right) => left.name.localeCompare(right.name));
}

export function useActivePlatformResourceDefinitionRecords(enabled = true) {
  const query = useResourceApiQuery<ResourceDefinitionVersionRecord[]>({
    api: async () => (await pageAllResourceDefinitions({
      definitionCode: null,
      definitionStatus: "ENABLED",
      lifecycleStatus: "ACTIVE",
      scopeKind: "PLATFORM",
      search: null,
      sourceKind: null,
      validationStatus: null,
      workspaceCode: null,
    })).map(adaptResourceDefinitionVersionRecord),
    deps: [enabled],
    enabled,
    initialData: () => [],
    mock: () => listPlatformResourceDefinitionVersionDetails()
      .filter((record) => record.lifecycleStatus === "ACTIVE" && record.enabled),
  });
  return {
    ...query,
    data: query.data.slice().sort((left, right) => left.name.localeCompare(right.name)),
  };
}
