import { getDashboardData } from "@/domain/dashboard/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { dashboardApiEndpoints } from "@/domain/dashboard/api";

export function useDashboardOverview() {
  return useServiceQuery({
    apiInitialData: () => ({ activities: [], metrics: [] }),
    domain: "dashboard",
    endpoint: dashboardApiEndpoints.getDashboardOverview,
    mockData: () => getDashboardData(),
    deps: [],
  });
}
