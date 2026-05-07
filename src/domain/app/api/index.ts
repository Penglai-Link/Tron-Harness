import {
  getApps,
  getImportApplicationDefaults,
} from "@/domain/app/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import type {
  AppRecord,
  ImportApplicationDefaults,
} from "@/domain/app/model/AppModel";

export type {
  AppRecord,
  ImportApplicationDefaults
} from "@/domain/app/model/AppModel";

export const appApiEndpoints = {
  listApps: "GET /api/v1/apps",
  getImportDefaults: "GET /api/v1/apps/import-defaults?source={source}",
} as const;

export function useApps() {
  return useServiceQuery({
    domain: "app",
    endpoint: appApiEndpoints.listApps,
    mockData: () => getApps(),
    deps: [],
  });
}

export function useImportApplicationDefaults(source: ImportApplicationDefaults["source"]) {
  return useServiceQuery({
    domain: "app",
    endpoint: appApiEndpoints.getImportDefaults,
    mockData: () => getImportApplicationDefaults(source),
    deps: [source],
    params: { source },
  });
}
