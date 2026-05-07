import { getDashboardData } from "@/domain/dashboard/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { dashboardApiEndpoints } from "@/domain/dashboard/api";

export function useDashboardData() {
  return useServiceQuery({
    domain: "dashboard",
    endpoint: dashboardApiEndpoints.getDashboardOverview,
    mockData: () => getDashboardData(),
    deps: [],
  });
}
