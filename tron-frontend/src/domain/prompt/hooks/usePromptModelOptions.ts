import {
  useEffect,
  useState,
} from "react";

import {
  fetchPlatformModels,
  fetchWorkspaceSelectOptions,
} from "@/domain/model/api";
import {
  getWorkspaceSelectOptions,
  listPlatformModelsMock,
} from "@/domain/model/mock";
import type { ModelSelectOption } from "@/domain/model/model";
import type { PromptScopeKind } from "@/domain/prompt/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function usePromptModelOptions(scopeKind: PromptScopeKind, workspaceCode: string) {
  const modelDataSource = getServiceDataSource("model");
  const [options, setOptions] = useState<ModelSelectOption[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    setOptions([]);
    if (scopeKind === "WORKSPACE" && !workspaceCode) return () => controller.abort();

    setLoading(true);
    const request = scopeKind === "WORKSPACE"
      ? (modelDataSource === "api"
        ? fetchWorkspaceSelectOptions(workspaceCode, controller.signal)
        : Promise.resolve(getWorkspaceSelectOptions(workspaceCode)))
      : (modelDataSource === "api"
        ? fetchPlatformModels(controller.signal)
        : Promise.resolve(listPlatformModelsMock()));

    request
      .then((records) => {
        if (controller.signal.aborted) return;
        const normalized: ModelSelectOption[] = records.map((record) => ({
          displayName: record.displayName,
          enabled: record.enabled,
          modelCode: record.modelCode,
          modelName: record.modelName,
          modelType: record.modelType,
          protocol: record.protocol,
          sourceKind: record.sourceKind,
          sourceLabel: record.sourceLabel,
          vendor: record.vendor,
        }));
        setOptions(normalized.filter((record) => record.enabled && record.modelType === "TEXT"));
      })
      .catch(() => {
        if (!controller.signal.aborted) setOptions([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [modelDataSource, scopeKind, workspaceCode]);

  return { loading, options };
}
