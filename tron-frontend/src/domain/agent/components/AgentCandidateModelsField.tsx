import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, TextInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";
import { fetchWorkspaceOptions } from "@/domain/admin/api";
import { fetchWorkspaceSelectOptions } from "@/domain/model/api";
import { getWorkspaceSelectOptions } from "@/domain/model/mock";
import type { ModelSelectOption } from "@/domain/model/model";
import { useWorkspaceSpaces } from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function AgentCandidateModelsField({ workspaceCode, value, onChange, disabled }: {
  workspaceCode: string;
  value: string[];
  onChange: (codes: string[]) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation("pages");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const catalogWorkspace = workspaceCode === "$platform"
    ? selectedSecondarySpace?.workspaceCode ?? selectedPrimarySpace?.workspaceCode ?? ""
    : workspaceCode;
  const [options, setOptions] = useState<ModelSelectOption[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const text = (key: string) => t(`workspaceAgents.internal.candidateModels.${key}`);
  useEffect(() => {
    if (!catalogWorkspace && workspaceCode !== "$platform") return;
    const controller = new AbortController();
    setLoading(true);
    setError(null);
    setOptions([]);
    const load = async () => {
      if (getServiceDataSource("model") !== "api") return getWorkspaceSelectOptions(catalogWorkspace);
      if (workspaceCode !== "$platform") return fetchWorkspaceSelectOptions(catalogWorkspace, controller.signal);
      const workspaces = await fetchWorkspaceOptions({ includeArchived: false }, controller.signal);
      const catalogs = await Promise.all(workspaces.map((workspace) => fetchWorkspaceSelectOptions(workspace.workspaceCode, controller.signal)));
      return [...new Map(catalogs.flat().map((option) => [option.optionCode || option.modelCode, option])).values()];
    };
    void load().then((records) => {
      if (!controller.signal.aborted) setOptions(records.filter((option) => option.modelType === "TEXT"
        && (!option.modelSelection || option.modelSelection.kind === "Fixed")
        && option.enabled && (option.contextWindowTokens ?? 0) > 0
        && (!option.remoteStatus || option.remoteStatus === "Available")));
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [catalogWorkspace, workspaceCode, revision]);
  const codes = new Set(options.map((option) => option.optionCode || option.modelCode));
  const unavailable = value.filter((code) => !codes.has(code));
  const query = search.trim().toLowerCase();
  const candidateModelsLabel = <span className="inline-flex items-center gap-1.5">
    <span>{text("label")}</span>
    <InfoTooltip ariaLabel={text("label")}>{text("help")}</InfoTooltip>
  </span>;
  return <Field label={candidateModelsLabel} actions={value.length ? (
    <Button type="button" size="sm" variant="ghost" disabled={disabled} onClick={() => onChange([])}>{text("clear")}</Button>
  ) : undefined}>
    <TextInput type="search" aria-label={text("search")} placeholder={text("search")} value={search} onChange={(event) => setSearch(event.target.value)} disabled={disabled || loading} />
    <div className="mt-2 w-full max-h-56 overflow-y-auto overscroll-y-contain rounded-md border border-[var(--color-outline-variant)] divide-y divide-[var(--color-outline-variant)]">
      {options.filter((option) => `${option.displayName} ${option.modelCode} ${option.vendor}`.toLowerCase().includes(query)).map((option) => {
        const code = option.optionCode || option.modelCode;
        return <label key={code} className="flex cursor-pointer items-center gap-3 px-3 py-2 text-sm hover:bg-[var(--color-surface-container)]">
          <input type="checkbox" aria-label={option.displayName} disabled={disabled} checked={value.includes(code)} onChange={(event) => onChange(event.target.checked ? [...value, code] : value.filter((item) => item !== code))} />
          <span className="min-w-0"><span className="block font-medium">{option.displayName}{option.qualityType ? ` · ${option.qualityType}` : ""}</span><span className="block break-all text-xs text-[var(--color-on-surface-variant)]">{option.sourceLabel} · {option.modelCode}</span></span>
        </label>;
      })}
      {!loading && !error && options.length === 0 ? <p className="p-3 text-sm text-[var(--color-on-surface-variant)]">{text(catalogWorkspace || workspaceCode === "$platform" ? "empty" : "noWorkspace")}</p> : null}
      {loading ? <p role="status" className="p-3 text-sm">{text("loading")}</p> : null}
    </div>
    {error ? <div className="mt-2 flex items-center gap-2"><p role="alert" className="text-sm text-[var(--color-error)]">{error}</p><Button type="button" size="sm" variant="outline" onClick={() => setRevision((current) => current + 1)}>{text("retry")}</Button></div> : null}
    {!loading && !error && unavailable.length ? <div className="mt-2 space-y-1">
      <p className="text-xs text-[var(--color-on-surface-variant)]">{text("unavailable")}</p>
      {unavailable.map((code) => <label key={code} className="flex items-center gap-2 text-xs"><input type="checkbox" checked disabled={disabled} onChange={() => onChange(value.filter((item) => item !== code))} /><span className="break-all">{code}</span></label>)}
    </div> : null}
  </Field>;
}
