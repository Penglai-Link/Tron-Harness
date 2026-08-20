import {
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  fetchRagCorpora,
  fetchRagCorpusDetail,
  fetchRagProfileCatalog,
  mapRagCorpusDetailToRecord,
  mapRagCorpusSummaryToRecord,
  normalizeRagProfileCatalog,
} from "@/domain/rag/api/ragManagementApi";
import { getRagMockCorpora } from "@/domain/rag/mock/data";
import type { CorpusRecord } from "@/domain/rag/model/RagModel";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface RagCorporaResult {
  corpora: CorpusRecord[];
  error: Error | null;
  loading: boolean;
  refresh: () => void;
}

function mergeCatalogCorpora(records: CorpusRecord[], catalogRecords: CorpusRecord[]) {
  const catalogByCode = new Map(catalogRecords.map((record) => [record.code, record]));
  return records.map((record) => {
    const catalogRecord = catalogByCode.get(record.code);
    if (!catalogRecord) return record;
    return {
      ...record,
      config: record.config ?? catalogRecord.config,
      filters: catalogRecord.filters.length > 0 ? catalogRecord.filters : record.filters,
    };
  });
}

async function fetchCorpusDetailRecords(records: CorpusRecord[], signal: AbortSignal) {
  const detailRecords = await Promise.all(records.map(async (record) => {
    try {
      return mapRagCorpusDetailToRecord(await fetchRagCorpusDetail(record.code, signal));
    } catch (error) {
      if (!signal.aborted) {
        console.warn(`[rag] Corpus detail request failed for ${record.code}.`, error);
      }
      return null;
    }
  }));
  const detailByCode = new Map(
    detailRecords
      .filter((record): record is CorpusRecord => Boolean(record))
      .map((record) => [record.code, record]),
  );
  return records.map((record) => detailByCode.get(record.code) ?? record);
}

export function useRagCorpora(): RagCorporaResult {
  const dataSource = getServiceDataSource("rag");
  const [reloadKey, setReloadKey] = useState(0);
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(false);
  const [corpora, setCorpora] = useState<CorpusRecord[]>(() => (
    dataSource === "api" ? [] : getRagMockCorpora()
  ));

  useEffect(() => {
    if (dataSource === "api") {
      setCorpora([]);
      setError(null);
      return;
    }
    setError(null);
    setCorpora(getRagMockCorpora());
  }, [dataSource, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api") return;
    const controller = new AbortController();
    setLoading(true);
    setError(null);

    fetchRagCorpora(controller.signal)
      .then(async (records) => {
        const summaryRecords = records.map(mapRagCorpusSummaryToRecord);
        const detailRecords = await fetchCorpusDetailRecords(summaryRecords, controller.signal);
        try {
          const catalog = normalizeRagProfileCatalog(await fetchRagProfileCatalog(undefined, controller.signal));
          if (!controller.signal.aborted) {
            setCorpora(mergeCatalogCorpora(detailRecords, catalog.corpora));
          }
        } catch (error) {
          if (!controller.signal.aborted) {
            console.warn("[rag] Corpus catalog request failed.", error);
            setCorpora(detailRecords);
          }
        }
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        const normalizedError = error instanceof Error
          ? error
          : new Error("RAG corpora could not be loaded.");
        console.warn("[rag] Corpus list request failed.", normalizedError);
        setError(normalizedError);
        setCorpora([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [dataSource, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    corpora,
    error,
    loading,
    refresh,
  };
}
