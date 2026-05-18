import { getAuditLogs } from "@/domain/admin/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { adminApiEndpoints } from "@/domain/admin/api";

const emptyAuditLogs = () => [];

export function useAuditLogList() {
  return useServiceQuery({
    apiInitialData: emptyAuditLogs,
    domain: "admin",
    endpoint: adminApiEndpoints.listAuditLogs,
    mockData: () => getAuditLogs(),
    deps: [],
  });
}
