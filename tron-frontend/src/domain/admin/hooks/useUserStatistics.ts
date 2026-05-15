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

export function useUserStatistics(): UserStatisticsResult {
  const dataSource = getServiceDataSource("admin");
  const mockStatistics = useMemo(() => getUserStatistics(), []);
  const [statistics, setStatistics] = useState<UserStatistics>(mockStatistics);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (dataSource === "api") return;
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
        console.warn("[admin] Falling back to mock user statistics.", error);
        setStatistics(mockStatistics);
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
