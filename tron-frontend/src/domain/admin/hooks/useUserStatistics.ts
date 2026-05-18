import {
  useEffect,
  useMemo,
  useState,
} from "react";

import { fetchAdminUserStatistics } from "@/domain/admin/api";
import { getUserStatistics } from "@/domain/admin/mock/data";
import type { UserStatistics } from "@/domain/admin/model";
import { isUnauthorizedApiError } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface UserStatisticsResult extends UserStatistics {
  loading: boolean;
}

const emptyStatistics: UserStatistics = {
  activeUsers: 0,
  roleAssignments: 0,
  spaceAssignments: 0,
  totalUsers: 0,
};

export function useUserStatistics(): UserStatisticsResult {
  const dataSource = getServiceDataSource("admin");
  const mockStatistics = useMemo(
    () => (dataSource === "api" ? emptyStatistics : getUserStatistics()),
    [dataSource],
  );
  const [statistics, setStatistics] = useState<UserStatistics>(() => (
    dataSource === "api" ? emptyStatistics : mockStatistics
  ));
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (dataSource === "api") {
      setStatistics(emptyStatistics);
      return;
    }
    setStatistics(mockStatistics);
  }, [dataSource, mockStatistics]);

  useEffect(() => {
    if (dataSource !== "api") return;

    const controller = new AbortController();
    setLoading(true);

    fetchAdminUserStatistics(controller.signal)
      .then((nextStatistics) => {
        setStatistics(nextStatistics);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        if (isUnauthorizedApiError(error)) return;
        console.warn("[admin] User statistics API request failed.", error);
        setStatistics(emptyStatistics);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, mockStatistics]);

  return {
    ...statistics,
    loading,
  };
}
