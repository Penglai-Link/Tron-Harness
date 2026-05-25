import {
  useEffect,
  useState,
} from "react";

import { fetchVariableCatalog } from "@/domain/variable/api";
import { listVariableCatalog } from "@/domain/variable/mock/data";
import type { VariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface VariableCatalogState {
  loading: boolean;
  refresh: () => void;
  variables: VariableRecord[];
}

export function useVariableCatalog(workspaceCode: string): VariableCatalogState {
  const dataSource = getServiceDataSource("variable");
  const [loading, setLoading] = useState(false);
  const [variables, setVariables] = useState<VariableRecord[]>(() => (
    workspaceCode && dataSource !== "api" ? listVariableCatalog(workspaceCode) : []
  ));
  const [refreshVersion, setRefreshVersion] = useState(0);

  useEffect(() => {
    if (!workspaceCode) {
      setVariables([]);
      setLoading(false);
      return;
    }

    if (dataSource !== "api") {
      setVariables(listVariableCatalog(workspaceCode));
      setLoading(false);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    fetchVariableCatalog(workspaceCode, {}, controller.signal)
      .then((records) => {
        setVariables(records);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[variable] Variable catalog request failed.", error);
        setVariables([]);
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
    variables,
  };
}
