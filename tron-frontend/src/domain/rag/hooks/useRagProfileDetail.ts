import {
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  fetchRagProfileDetail,
  mapRagProfileDetailToRecord,
  type RagProfileDetailApi,
} from "@/domain/rag/api/ragManagementApi";
import { findRagMockProfile } from "@/domain/rag/mock/data";
import type { ProfileRecord } from "@/domain/rag/model/RagModel";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface RagProfileDetailResult {
  detail: RagProfileDetailApi | null;
  loading: boolean;
  profile: ProfileRecord | null;
  refresh: () => void;
}

export function useRagProfileDetail(profileId: string): RagProfileDetailResult {
  const dataSource = getServiceDataSource("rag");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [detail, setDetail] = useState<RagProfileDetailApi | null>(null);
  const [profile, setProfile] = useState<ProfileRecord | null>(() => {
    if (dataSource === "api" || !profileId.trim()) return null;
    return findRagMockProfile(profileId) ?? null;
  });

  useEffect(() => {
    if (!profileId.trim()) {
      setDetail(null);
      setProfile(null);
      return;
    }

    if (dataSource === "api") return;
    setDetail(null);
    setProfile(findRagMockProfile(profileId) ?? null);
  }, [dataSource, profileId, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api" || !profileId.trim()) return;
    const controller = new AbortController();
    setLoading(true);
    setDetail(null);
    setProfile(null);

    fetchRagProfileDetail(profileId, controller.signal)
      .then((value) => {
        setDetail(value);
        setProfile(mapRagProfileDetailToRecord(value));
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[rag] Profile detail request failed.", error);
        setDetail(null);
        setProfile(null);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, profileId, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    detail,
    loading,
    profile,
    refresh,
  };
}
