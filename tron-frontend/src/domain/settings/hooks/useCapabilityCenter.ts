import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import {
  notifyCapabilityResourcesChanged,
  CAPABILITY_RESOURCES_CHANGED,
} from "@/domain/skill/api";
import {
  getCenterCategories,
  getCenterPage,
  installCenterRelease,
  isCenterReleaseUnavailable,
  type CenterQuery,
  type CenterPack,
} from "../api/capabilityCenterApi";
import {
  importCapabilityPack,
  setCapabilityPackEnabled,
  uninstallCapabilityPack,
} from "../api/capabilityApi";

export type CenterAction =
  | { kind: "install"; releaseId: string; installedVersion: string | null }
  | { kind: "import"; file: File }
  | { kind: "toggle" | "uninstall"; pack: CenterPack };

export function useCapabilityCenter() {
  const { t, i18n } = useTranslation("pages");
  const toast = useToast();
  const client = useQueryClient();
  const lock = useRef(false);
  useEffect(() => {
    const changed = () => {
      if (!lock.current)
        void client.invalidateQueries({ queryKey: ["capability-center"] });
    };
    window.addEventListener(CAPABILITY_RESOURCES_CHANGED, changed);
    return () =>
      window.removeEventListener(CAPABILITY_RESOURCES_CHANGED, changed);
  }, [client]);
  const [query, setQuery] = useState<CenterQuery>({
    current: 1,
    size: 20,
    keyword: "",
    category: "",
    source: "",
    status: "all",
  });
  const filters = useCallback(
    (patch: Partial<CenterQuery>) =>
      setQuery((q) => ({ ...q, ...patch, current: patch.current ?? 1 })),
    []
  );
  const search = useCallback(
    (keyword: string) => filters({ keyword }),
    [filters]
  );
  const page = useQuery({
    queryKey: ["capability-center", "page", i18n.resolvedLanguage, query],
    queryFn: ({ signal }) => getCenterPage(query, signal),
    retry: false,
  });
  const categories = useQuery({
    queryKey: ["capability-center", "categories"],
    queryFn: ({ signal }) => getCenterCategories(signal),
    retry: false,
    staleTime: 60_000,
  });
  useEffect(() => {
    if (
      page.data &&
      query.current > 1 &&
      (query.current - 1) * query.size >= page.data.total
    ) {
      filters({
        current: Math.max(1, Math.ceil(page.data.total / query.size)),
      });
    }
  }, [page.data, query.current, query.size, filters]);
  const mutation = useMutation({
    mutationFn: async (action: CenterAction) => {
      if (action.kind === "install")
        return installCenterRelease(action.releaseId, action.installedVersion);
      if (action.kind === "import") return importCapabilityPack(action.file);
      if (action.kind === "uninstall")
        return uninstallCapabilityPack(action.pack.packId);
      return setCapabilityPackEnabled(action.pack.packId, !action.pack.enabled);
    },
    onSuccess: (_, action) => {
      const key =
        action.kind === "install"
          ? "installedNotice"
          : action.kind === "import"
          ? "importedNotice"
          : action.kind === "uninstall"
          ? "uninstalledNotice"
          : action.pack.enabled
          ? "disabledNotice"
          : "enabledNotice";
      toast.success(t(`capabilityCenter.${key}`));
    },
    onError: (error) =>
      toast.error(error.message, {
        title: t("capabilityCenter.operationFailed"),
      }),
    onSettled: async (_, error) => {
      // Reconcile even after a partial runtime failure; never leave an optimistic installed state.
      notifyCapabilityResourcesChanged();
      window.dispatchEvent(new Event("tron:navigation-updated"));
      await client.invalidateQueries({
        queryKey: ["capability-center"],
        predicate: (query) =>
          !isCenterReleaseUnavailable(error) || query.queryKey[1] !== "detail",
      });
    },
  });
  const execute = async (action: CenterAction) => {
    if (lock.current) return false;
    lock.current = true;
    try {
      await mutation.mutateAsync(action);
      return true;
    } catch {
      return false;
    } finally {
      lock.current = false;
    }
  };
  const refresh = () =>
    client.invalidateQueries({ queryKey: ["capability-center"] });
  return {
    query,
    filters,
    search,
    page,
    categories,
    mutation,
    execute,
    refresh,
  };
}
