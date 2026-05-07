import { listRoles } from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";

export function useRoleList() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listRoles,
    mockData: () => listRoles(),
    deps: [],
  });
}
