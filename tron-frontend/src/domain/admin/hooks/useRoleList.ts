import { listRoles } from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";

const emptyRoles = () => [];

export function useRoleList() {
  return useServiceQuery({
    apiInitialData: emptyRoles,
    domain: "admin",
    endpoint: adminApiEndpoints.listRoles,
    mockData: () => listRoles(),
    deps: [],
  });
}
