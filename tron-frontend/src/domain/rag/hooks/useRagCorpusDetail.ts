import {
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  fetchRagCorpusDetail,
  mapRagCorpusDetailToRecord,
  type RagCorpusDetailApi,
} from "@/domain/rag/api/ragManagementApi";
import { findRagMockCorpus } from "@/domain/rag/mock/data";
import type { CorpusRecord } from "@/domain/rag/model/RagModel";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface RagCorpusDetailResult {
  detail: RagCorpusDetailApi | null;
  loading: boolean;
  record: CorpusRecord | null;
  refresh: () => void;
}

export function useRagCorpusDetail(corpusId: string): RagCorpusDetailResult {
  const dataSource = getServiceDataSource("rag");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [detail, setDetail] = useState<RagCorpusDetailApi | null>(null);
  const [record, setRecord] = useState<CorpusRecord | null>(() => {
    if (dataSource === "api" || !corpusId.trim()) return null;
    return findRagMockCorpus(corpusId) ?? null;
  });

  useEffect(() => {
    if (!corpusId.trim()) {
      setDetail(null);
      setRecord(null);
      return;
    }

    if (dataSource === "api") return;
    setDetail(null);
    setRecord(findRagMockCorpus(corpusId) ?? null);
  }, [corpusId, dataSource, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api" || !corpusId.trim()) return;
    const controller = new AbortController();
    setLoading(true);
    setDetail(null);
    setRecord(null);

    fetchRagCorpusDetail(corpusId, controller.signal)
      .then((value) => {
        setDetail(value);
        setRecord(mapRagCorpusDetailToRecord(value));
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[rag] Corpus detail request failed.", error);
        setDetail(null);
        setRecord(null);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [corpusId, dataSource, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    detail,
    loading,
    record,
    refresh,
  };
}
