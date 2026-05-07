import { getImportApplicationDefaults } from "@/domain/app/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { appApiEndpoints } from "@/domain/app/api";
import type { ImportApplicationDefaults } from "@/domain/app/model";

export function useImportApplicationDefaults(source: ImportApplicationDefaults["source"]) {
  return useServiceQuery({
    domain: "app",
    endpoint: appApiEndpoints.getImportDefaults,
    mockData: () => getImportApplicationDefaults(source),
    deps: [source],
    params: { source },
  });
}
