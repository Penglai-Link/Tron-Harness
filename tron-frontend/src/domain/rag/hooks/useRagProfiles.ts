import {
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  fetchRagProfiles,
  mapRagProfileDetailToRecord,
} from "@/domain/rag/api/ragManagementApi";
import { getRagMockProfiles } from "@/domain/rag/mock/data";
import type { ProfileRecord } from "@/domain/rag/model/RagModel";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface RagProfilesResult {
  loading: boolean;
  profiles: ProfileRecord[];
  refresh: () => void;
}

export function useRagProfiles(): RagProfilesResult {
  const dataSource = getServiceDataSource("rag");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [profiles, setProfiles] = useState<ProfileRecord[]>(() => (
    dataSource === "api" ? [] : getRagMockProfiles()
  ));

  useEffect(() => {
    if (dataSource === "api") {
      setProfiles([]);
      return;
    }
    setProfiles(getRagMockProfiles());
  }, [dataSource, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);

    fetchRagProfiles(controller.signal)
      .then((records) => setProfiles(records.map(mapRagProfileDetailToRecord)))
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[rag] Profile list request failed.", error);
        setProfiles([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    loading,
    profiles,
    refresh,
  };
}
