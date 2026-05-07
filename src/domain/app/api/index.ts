import {
  getApps,
  getImportApplicationDefaults,
} from "@/domain/app/mock/data";
import { useServiceQuery } from "@/domain/service-config";
import type {
  AppRecord,
  ImportApplicationDefaults,
} from "@/domain/app/model/model";

export type {
  AppRecord,
  ImportApplicationDefaults
} from "@/domain/app/model/model";

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
