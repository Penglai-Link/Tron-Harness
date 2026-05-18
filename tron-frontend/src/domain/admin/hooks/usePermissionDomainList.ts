import { listPermissionDomains } from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";

const emptyPermissionDomains = () => [];

export function usePermissionDomainList() {
  return useServiceQuery({
    apiInitialData: emptyPermissionDomains,
    domain: "admin",
    endpoint: adminApiEndpoints.listPermissionDomains,
    mockData: () => listPermissionDomains(),
    deps: [],
  });
}
