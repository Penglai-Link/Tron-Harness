import { getAuditLogs } from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";

export function useAuditLogList() {
  return useServiceQuery({
    domain: "admin",
    endpoint: adminApiEndpoints.listAuditLogs,
    mockData: () => getAuditLogs(),
    deps: [],
  });
}
