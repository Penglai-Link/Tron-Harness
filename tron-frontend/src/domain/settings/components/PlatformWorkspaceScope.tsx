import { useToast } from "@/components/ui/toast";
import { createContext, useContext, useEffect, useRef, useMemo, useCallback, type ReactNode } from "react";
import { useNavigate, useSearchParams, type NavigateOptions } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { fetchWorkspaceOptions } from "@/domain/admin/api";
import { useAuthStore } from "@/store/authStore";
import { WorkspaceSpacesContext, useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { localizeWorkspaceOptions } from "../constants/workspacePresentation";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";

export function usePlatformWorkspaces() {
  const { t } = useTranslation("pages");
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const select = useCallback((options: WorkspaceOption[]) => localizeWorkspaceOptions(options, t("platformData.defaultWorkspace")), [t]);
  return useQuery({ queryKey: ["platform-data-workspaces", user], queryFn: ({ signal }) => fetchWorkspaceOptions({}, signal), select });
}

const WorkspaceFilterContext = createContext<ReactNode>(null);
export function PlatformWorkspaceFilter() { return useContext(WorkspaceFilterContext); }

export function PlatformWorkspaceScope({ children, renderWithoutWorkspace = false, allowAll = false, showScopeHelp = true, inlineFilter = false }: { children: ReactNode; renderWithoutWorkspace?: boolean; allowAll?: boolean; showScopeHelp?: boolean; inlineFilter?: boolean }) {
  const { t } = useTranslation("pages");
  const query = usePlatformWorkspaces();
  const toast = useToast();
  const notifiedError = useRef<string | null>(null);
  useEffect(() => {
    const message = query.error?.message ?? null;
    if (allowAll && message && message !== notifiedError.current) toast.error(message);
    notifiedError.current = message;
  }, [allowAll, query.error, toast]);
  const [params, setParams] = useSearchParams();
  const options = (query.data ?? []).filter((option) => allowAll || option.workspaceCode !== "PLATFORM");
  const requested = params.get("workspaceCode");
  const selected = requested ? options.find((option) => option.workspaceCode === requested) : allowAll ? undefined : options[0];
  const workspace = selected?.workspaceCode ?? "";
  const context = useMemo(() => ({
    primarySpaces: [], secondarySpaces: [], selectedPrimarySpaceId: workspace, selectedSecondarySpaceId: "",
    selectedPrimarySpace: selected ? { ...selected, childCount: 0 } : undefined,
    selectedSecondarySpace: undefined,
  }), [selected, workspace]);
  const scopeControl = (
    <div className="flex shrink-0 items-center gap-2">
      <SelectInput aria-label={t(allowAll ? "observability:workspace" : "platformData.workspace")} className={inlineFilter ? "w-48 max-w-full" : "w-full sm:w-72"} value={workspace || (allowAll ? "__all__" : "__none__")} disabled={query.isLoading} onValueChange={(value) => {
        if (allowAll && value === "__all__") { const next = new URLSearchParams(params); next.delete("workspaceCode"); setParams(next); return; }
        if (!options.some((option) => option.workspaceCode === value)) return;
        const next = new URLSearchParams(params); next.set("workspaceCode", String(value)); setParams(next);
      }}>
        {allowAll ? <SelectItem value="__all__">{t("observability:allWorkspaces")}</SelectItem> : null}
        {!workspace && !allowAll ? <SelectItem value="__none__">{t("platformData.selectWorkspace")}</SelectItem> : null}
        {options.map((option) => <SelectItem key={option.workspaceCode} value={option.workspaceCode}>{option.label || option.name}</SelectItem>)}
      </SelectInput>
      {allowAll && showScopeHelp ? <InfoTooltip variant="icon" ariaLabel={t("observability:workspaceScopeHelpLabel")}>{t("observability:workspaceScopeHelp")}</InfoTooltip> : null}
    </div>
  );
  return <div className="flex h-full min-h-0 flex-col gap-4">
    {!inlineFilter && scopeControl}
    {query.error && !allowAll ? <p role="alert">{query.error.message}</p> : null}
    {(selected || renderWithoutWorkspace || (allowAll && !requested)) ? <WorkspaceSpacesContext.Provider value={context}><div key={workspace} className="min-h-0 flex-1"><WorkspaceFilterContext.Provider value={scopeControl}>{children}</WorkspaceFilterContext.Provider></div></WorkspaceSpacesContext.Provider> : <p role="status">{t("platformData.selectWorkspace")}</p>}
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
