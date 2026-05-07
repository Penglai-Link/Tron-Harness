import { getApps } from "@/domain/app/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { appApiEndpoints } from "@/domain/app/api";

export function useApps() {
  return useServiceQuery({
    domain: "app",
    endpoint: appApiEndpoints.listApps,
    mockData: () => getApps(),
    deps: [],
  });
}
