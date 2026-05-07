import { listUsers } from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";

export function useUsers() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listUsers,
    mockData: () => listUsers(),
    deps: [],
  });
}
