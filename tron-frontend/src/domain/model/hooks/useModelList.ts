import {
  useEffect,
  useState,
} from "react";

import { fetchWorkspaceModels } from "@/domain/model/api";
import { listWorkspaceModels } from "@/domain/model/mock/data";
import type { WorkspaceModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface WorkspaceModelCatalogState {
  loading: boolean;
  refresh: () => void;
  models: WorkspaceModelRecord[];
}

export function useModelList(workspaceCode: string): WorkspaceModelCatalogState {
  const dataSource = getServiceDataSource("model");
  const [loading, setLoading] = useState(false);
  const [models, setModels] = useState<WorkspaceModelRecord[]>(() => (
    workspaceCode ? listWorkspaceModels(workspaceCode) : []
  ));
  const [refreshVersion, setRefreshVersion] = useState(0);

  useEffect(() => {
    if (!workspaceCode) {
      setModels([]);
      setLoading(false);
      return;
    }

    if (dataSource !== "api") {
      setModels(listWorkspaceModels(workspaceCode));
      setLoading(false);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    fetchWorkspaceModels(workspaceCode, {}, controller.signal)
      .then((records) => {
        setModels(records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[model] Falling back to mock registry.", error);
        setModels(listWorkspaceModels(workspaceCode));
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, [dataSource, refreshVersion, workspaceCode]);

  return {
    loading,
    refresh: () => setRefreshVersion((value) => value + 1),
    models,
  };
}
