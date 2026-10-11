import { useEffect, useState } from "react";
import { fetchPromptDetail } from "@/domain/prompt/api";
import { getPlatformPromptMock } from "@/domain/prompt/mock";
import type { PromptRecord } from "@/domain/prompt/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function usePromptTemplateDetail(promptCode: string) {
  const dataSource = getServiceDataSource("prompt");
  const [record, setRecord] = useState<PromptRecord | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [reload, setReload] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setRecord(null);
    setError(null);
    setLoading(false);
    if (!promptCode) return () => controller.abort();

    setLoading(true);
    const request = Promise.resolve().then(() => dataSource === "api"
      ? fetchPromptDetail({ promptCode }, controller.signal)
      : getPlatformPromptMock({ promptCode }));
    request.then((next) => { if (!controller.signal.aborted) setRecord(next); })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) setError(reason instanceof Error ? reason : new Error(String(reason)));
      })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [dataSource, promptCode, reload]);

  return { record: record?.promptCode === promptCode ? record : null, loading, error, refresh: () => setReload((value) => value + 1) };
}
