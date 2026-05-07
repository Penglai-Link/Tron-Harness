import { getDashboardData } from "@/domain/dashboard/mock/data";

import { useServiceQuery } from "@/shared/api/serviceConfig";
import type {
  DashboardMetric,
  DashboardActivity,
} from "@/domain/dashboard/model/DashboardModel";

export type {
  DashboardMetric,
  DashboardActivity
} from "@/domain/dashboard/model/DashboardModel";

export const dashboardApiEndpoints = {
  getDashboardOverview: "GET /api/v1/dashboard/overview",
} as const;

export function useDashboardData() {
  return useServiceQuery({
    domain: "dashboard",
    endpoint: dashboardApiEndpoints.getDashboardOverview,
    mockData: () => getDashboardData(),
    deps: [],
  });
}
