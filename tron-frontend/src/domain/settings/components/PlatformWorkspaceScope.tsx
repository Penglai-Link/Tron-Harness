import { useMemo, type ReactNode } from "react";
import { useNavigate, useSearchParams, type NavigateOptions } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { fetchWorkspaceOptions } from "@/domain/admin/api";
import { useAuthStore } from "@/store/authStore";
import { WorkspaceSpacesContext, useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";

export function usePlatformWorkspaces() {
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  return useQuery({ queryKey: ["platform-data-workspaces", user], queryFn: ({ signal }) => fetchWorkspaceOptions({}, signal) });
}

export function PlatformWorkspaceScope({ children }: { children: ReactNode }) {
  const { t } = useTranslation("pages");
  const query = usePlatformWorkspaces();
  const [params, setParams] = useSearchParams();
  const options = (query.data ?? []).filter((option) => option.workspaceCode !== "PLATFORM");
  const requested = params.get("workspaceCode");
  const selected = requested ? options.find((option) => option.workspaceCode === requested) : options[0];
  const workspace = selected?.workspaceCode ?? "";
  const context = useMemo(() => ({
    primarySpaces: [], secondarySpaces: [], selectedPrimarySpaceId: workspace, selectedSecondarySpaceId: "",
    selectedPrimarySpace: selected ? { ...selected, childCount: 0 } : undefined,
    selectedSecondarySpace: undefined,
  }), [selected, workspace]);
  return <div className="flex h-full min-h-0 flex-col gap-4">
    <div className="shrink-0">
      <SelectInput aria-label={t("platformData.workspace")} className="w-full sm:w-72" value={workspace || "__none__"} disabled={query.isLoading} onValueChange={(value) => {
        const next = new URLSearchParams(params); next.set("workspaceCode", String(value)); setParams(next);
      }}>
        {!workspace ? <SelectItem value="__none__">{t("platformData.selectWorkspace")}</SelectItem> : null}
        {options.map((option) => <SelectItem key={option.workspaceCode} value={option.workspaceCode}>{option.label || option.name}</SelectItem>)}
      </SelectInput>
    </div>
    {query.error ? <p role="alert">{query.error.message}</p> : null}
    {selected ? <WorkspaceSpacesContext.Provider value={context}><div key={workspace} className="min-h-0 flex-1">{children}</div></WorkspaceSpacesContext.Provider> : <p role="status">{t("platformData.selectWorkspace")}</p>}
  </div>;
}

export function usePlatformDataHref() {
  const [params] = useSearchParams();
  const spaces = useWorkspaceSpaces();
  return (path: string) => {
    const url = new URL(path, "https://tron.local");
    if (/^\/(memory|knowledge-base|knowledge-graph|rag)(\/|$)/.test(url.pathname)) url.pathname = `/settings${url.pathname}`;
    const workspace = params.get("workspaceCode") ?? spaces.selectedPrimarySpace?.workspaceCode;
    if (workspace && !url.pathname.startsWith("/settings/knowledge-base") && !url.searchParams.has("workspaceCode")) url.searchParams.set("workspaceCode", workspace);
    return `${url.pathname}${url.search}${url.hash}`;
  };
}

export function usePlatformDataNavigate() {
  const navigate = useNavigate();
  const href = usePlatformDataHref();
  return (path: string, options?: NavigateOptions) => navigate(href(path), options);
}
