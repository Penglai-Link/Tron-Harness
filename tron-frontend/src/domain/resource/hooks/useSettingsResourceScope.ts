import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { useLocation } from "react-router";

import { fetchWorkspaceOptions } from "@/domain/admin/api";
import { getWorkspaceOptionMocks } from "@/shared/constants/workspaceSpaces";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useWorkspaceSpaces } from "@/shared";

const settingsResourceWorkspaceStorageKey = "tron.settings.resource.workspaceCode";

function readStoredWorkspaceCode() {
  if (typeof window === "undefined") return "";
  return window.sessionStorage.getItem(settingsResourceWorkspaceStorageKey) ?? "";
}

function storeWorkspaceCode(workspaceCode: string) {
  if (typeof window === "undefined") return;
  window.sessionStorage.setItem(settingsResourceWorkspaceStorageKey, workspaceCode);
}

export interface SettingsResourceScope {
  isSettings: boolean;
  options: WorkspaceOption[];
  selectWorkspace: (workspaceCode: string) => void;
  workspaceCode: string;
  workspaceName: string;
  workspacesLoading: boolean;
}

export function useSettingsResourceScope(): SettingsResourceScope {
  const location = useLocation();
  const isSettings = location.pathname.startsWith("/settings/");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const consoleWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const [options, setOptions] = useState<WorkspaceOption[]>([]);
  const [selectedCode, setSelectedCode] = useState(readStoredWorkspaceCode);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isSettings) return;
    if (getServiceDataSource("resource") !== "api") {
      const mocks = getWorkspaceOptionMocks();
      setOptions(mocks);
      setSelectedCode((current) => {
        if (current && mocks.some((option) => option.workspaceCode === current)) {
          return current;
        }
        const fallback = mocks[0]?.workspaceCode ?? "";
        if (fallback) storeWorkspaceCode(fallback);
        return fallback;
      });
      return;
    }
    const controller = new AbortController();
    setLoading(true);
    fetchWorkspaceOptions({ status: "Active" }, controller.signal)
      .then((fetched) => {
        if (controller.signal.aborted) return;
        setOptions(fetched);
        setSelectedCode((current) => {
          if (current && fetched.some((option) => option.workspaceCode === current)) {
            return current;
          }
          const fallback = fetched[0]?.workspaceCode ?? "";
          if (fallback) storeWorkspaceCode(fallback);
          return fallback;
        });
      })
      .catch(() => {
        if (!controller.signal.aborted) setOptions([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [isSettings]);

  const selectWorkspace = useCallback((workspaceCode: string) => {
    setSelectedCode(workspaceCode);
    storeWorkspaceCode(workspaceCode);
  }, []);

  return useMemo(() => {
    if (!isSettings) {
      return {
        isSettings,
        options: [],
        selectWorkspace,
        workspaceCode: consoleWorkspace?.workspaceCode ?? "",
        workspaceName: consoleWorkspace?.name ?? "",
        workspacesLoading: false,
      };
    }
    const selected = options.find((option) => option.workspaceCode === selectedCode);
    return {
      isSettings,
      options,
      selectWorkspace,
      workspaceCode: selected?.workspaceCode ?? "",
      workspaceName: selected?.name ?? "",
      workspacesLoading: loading,
    };
  }, [consoleWorkspace, isSettings, loading, options, selectWorkspace, selectedCode]);
}
