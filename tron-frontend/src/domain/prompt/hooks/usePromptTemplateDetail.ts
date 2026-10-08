import { useEffect, useState } from "react";
import { fetchPlatformPromptDetail, fetchWorkspacePromptDetail } from "@/domain/prompt/api";
import { getPlatformPromptMock, getWorkspacePromptMock } from "@/domain/prompt/mock";
import type { PromptRecord, PromptScopeKind } from "@/domain/prompt/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function usePromptTemplateDetail(scopeKind: PromptScopeKind, workspaceCode: string | undefined, promptCode: string) {
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
    if (!promptCode || (scopeKind === "WORKSPACE" && !workspaceCode)) return () => controller.abort();

    setLoading(true);
    // Resolve mocks inside the promise so missing records follow the same error path as HTTP.
    const request = Promise.resolve().then(() => scopeKind === "PLATFORM"
      ? (dataSource === "api"
        ? fetchPlatformPromptDetail({ promptCode }, controller.signal)
        : getPlatformPromptMock({ promptCode }))
      : (dataSource === "api"
        ? fetchWorkspacePromptDetail({ promptCode, workspaceCode: workspaceCode! }, controller.signal)
        : getWorkspacePromptMock({ promptCode, workspaceCode: workspaceCode! })));
    request.then((next) => {
      if (!controller.signal.aborted) setRecord(next);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason : new Error(String(reason)));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [dataSource, promptCode, reload, scopeKind, workspaceCode]);

  // Hide a previous selection synchronously, before the new effect has run.
  return { record: record?.promptCode === promptCode ? record : null, loading, error, refresh: () => setReload((value) => value + 1) };
}
