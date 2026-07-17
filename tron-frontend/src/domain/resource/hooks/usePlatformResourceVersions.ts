import { useMemo } from "react";

import {
  listPlatformResourceVersionDetails,
  listPlatformResourceVersions,
} from "@/domain/resource/mock";

export function usePlatformResourceList() {
  return useMemo(() => listPlatformResourceVersions(), []);
}

export function usePlatformResourceVersionDetails(resourceCode: string) {
  return useMemo(() => (
    listPlatformResourceVersionDetails()
      .filter((record) => record.resourceCode === resourceCode)
      .sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt))
  ), [resourceCode]);
}
