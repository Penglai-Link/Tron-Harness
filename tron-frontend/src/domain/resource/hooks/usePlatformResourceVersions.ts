import { useMemo } from "react";

import {
  listPlatformResourceVersionDetails,
  listPlatformResourceVersions,
} from "@/domain/resource/mock";

import { adaptResourceVersionDetail } from "@/domain/resource/adapters";
import { getResource, pageResources } from "@/domain/resource/api";

import { useResourceApiQuery } from "./useResourceApiQuery";

export function usePlatformResourceList() {
  return useMemo(() => listPlatformResourceVersions(), []);
}

export function usePlatformResourceVersionDetails(resourceCode: string) {
  const query = useResourceApiQuery({
    api: async () => {
      const page = await pageResources({
        current: 1,
        definitionCode: null,
        definitionStatus: null,
        lifecycleStatus: null,
        resourceCode,
        scopeKind: "PLATFORM",
        search: null,
        size: 50,
        sourceKind: null,
        validationStatus: null,
        workspaceCode: null,
      });
      return Promise.all(page.records.map(async (record) => (
        adaptResourceVersionDetail(await getResource({
          resourceCode: record.resourceCode,
          resourceVersion: record.resourceVersion,
          scopeKind: "PLATFORM",
          workspaceCode: null,
        }))
      )));
    },
    deps: [resourceCode],
    enabled: Boolean(resourceCode),
    initialData: () => [],
    mock: () => listPlatformResourceVersionDetails()
      .filter((record) => record.resourceCode === resourceCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt)),
  });
  return query.data;
}
