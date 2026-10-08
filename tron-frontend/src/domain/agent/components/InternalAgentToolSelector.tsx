import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  PlatformToolRecord,
  ToolCatalogDomainRecord,
} from "@/domain/agent/model";
import { cn } from "@/shared/lib/utils";
import { Field } from "@/components/forms";
import { SearchField } from "@/components/search-field";
import { Button } from "@/components/ui/button";

interface InternalAgentToolSelectorProps {
  className?: string;
  busy?: boolean;
  onSetToolsChecked: (toolCodes: string[], checked: boolean) => void;
  onToggleTool: (toolCode: string, checked: boolean) => void;
  selectedToolCodes: string[];
  toolDomains: ToolCatalogDomainRecord[];
  tools: PlatformToolRecord[];
}

interface DomainEntry {
  domain: ToolCatalogDomainRecord;
  selectedCount: number;
  visibleTools: PlatformToolRecord[];
}

function normalizeSearchText(value: string) {
  return value.trim().toLowerCase();
}

export function InternalAgentToolSelector({
  className,
  busy = false,
  onSetToolsChecked,
  onToggleTool,
  selectedToolCodes,
  toolDomains,
  tools,
}: InternalAgentToolSelectorProps) {
  const { t } = useTranslation(["pages", "common"]);
  const [toolSearch, setToolSearch] = useState("");
  const [debouncedToolSearch, setDebouncedToolSearch] = useState("");
  const [activeDomainCode, setActiveDomainCode] = useState("");
  const selectedToolSet = useMemo(() => new Set(selectedToolCodes), [selectedToolCodes]);

  const internalPageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.internal.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedToolSearch(normalizeSearchText(toolSearch));
    }, 180);

    return () => window.clearTimeout(timer);
  }, [toolSearch]);

  const allToolsByDomain = useMemo(() => {
    const grouped = new Map<string, PlatformToolRecord[]>();
    tools.forEach((tool) => {
      const current = grouped.get(tool.domainCode) ?? [];
      grouped.set(tool.domainCode, [...current, tool]);
    });

    grouped.forEach((items, domainCode) => {
      grouped.set(
        domainCode,
        [...items].sort((left, right) => left.toolCode.localeCompare(right.toolCode)),
      );
    });

    return grouped;
  }, [tools]);

  const domainEntries = useMemo(() => {
    const normalizedSearch = debouncedToolSearch;

    return toolDomains.reduce<DomainEntry[]>((accum, domain) => {
      const domainTools = allToolsByDomain.get(domain.domainCode) ?? [];
      if (!domainTools.length) {
        return accum;
      }

      const matchesDomain = normalizedSearch.length === 0
        || [
          domain.domainCode,
          domain.name,
          domain.description,
          domain.sampleTools.join(" "),
        ].join(" ").toLowerCase().includes(normalizedSearch);

      const visibleTools = matchesDomain
        ? domainTools
        : domainTools.filter((tool) => (
          [
            tool.toolCode,
            tool.domainCode,
            tool.name,
            tool.description,
            tool.operationKind,
            t(`common:toolOperationKind.${tool.operationKind}`, { defaultValue: tool.operationKind }),
          ].join(" ").toLowerCase().includes(normalizedSearch)
        ));

      if (!visibleTools.length) {
        return accum;
      }

      accum.push({
        domain,
        selectedCount: domainTools.filter((tool) => selectedToolSet.has(tool.toolCode)).length,
        visibleTools,
      });

      return accum;
    }, []);
  }, [allToolsByDomain, debouncedToolSearch, selectedToolSet, t, toolDomains]);

  useEffect(() => {
    if (domainEntries.length === 0) {
      if (activeDomainCode) {
        setActiveDomainCode("");
      }
      return;
    }

    const activeStillVisible = domainEntries.some((entry) => entry.domain.domainCode === activeDomainCode);
    if (!activeStillVisible) {
      setActiveDomainCode(domainEntries[0].domain.domainCode);
    }
  }, [activeDomainCode, domainEntries]);

  const activeEntry = useMemo(() => (
    domainEntries.find((entry) => entry.domain.domainCode === activeDomainCode) ?? null
  ), [activeDomainCode, domainEntries]);

  const activeDomainTools = activeEntry?.visibleTools ?? [];
  const allActiveToolsSelected = activeDomainTools.length > 0
    && activeDomainTools.every((tool) => selectedToolSet.has(tool.toolCode));
  const hasAnyActiveToolSelected = activeDomainTools.some((tool) => selectedToolSet.has(tool.toolCode));

  return (
    <div className={cn("mt-5 space-y-3", className)}>
      <div className="grid gap-4 md:grid-cols-[minmax(180px,240px)_minmax(0,1fr)]">
        <Field className="w-full min-w-0 pr-1" label={internalPageT("toolSelection.title", "Selected tools")}>
          <SearchField
            className="w-full"
            ariaLabel={internalPageT("toolSelection.searchAria", "Search tools")}
            placeholder={internalPageT("toolSelection.searchPlaceholder", "Search tools...")}
            value={toolSearch}
            onChange={setToolSearch}
          />
        </Field>
      </div>

      {domainEntries.length === 0 ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {internalPageT("toolSelection.empty", "No tools match the search query.")}
        </p>
      ) : (
        <div className="grid gap-4 md:grid-cols-[minmax(180px,240px)_minmax(0,1fr)]">
          <div>
            <div className="tron-scrollbar max-h-[36rem] space-y-2 overflow-x-hidden overflow-y-auto overscroll-y-contain pr-1">
              {domainEntries.map((entry) => {
                const isActive = entry.domain.domainCode === activeDomainCode;

                return (
                  <button
                    key={entry.domain.domainCode}
                    className={cn(
                      "w-full rounded-[18px] border p-3 text-left shadow-[0_8px_18px_rgba(15,23,42,0.03)] transition-[border-color,background-color,box-shadow]",
                      isActive
                        ? "border-[var(--color-primary)] bg-[var(--color-primary)]/8 shadow-[0_12px_24px_rgba(15,23,42,0.06)]"
                        : "border-[var(--color-card-border)] bg-[var(--color-surface-container-lowest)] hover:border-[var(--color-outline-variant)] hover:bg-[var(--color-hover-surface)]",
                    )}
                    type="button"
                    onClick={() => {
                      setActiveDomainCode(entry.domain.domainCode);
                    }}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium text-[var(--color-on-surface)]">
                          {entry.domain.name}
                        </p>
                      </div>
                      <span className="rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">
                        {internalPageT("toolSelection.selectedCount", "{{count}} selected", {
                          count: entry.selectedCount,
                        })}
                      </span>
                    </div>
                    <p className="mt-2 line-clamp-2 text-[12px] text-[var(--color-on-surface-variant)]">
                      {entry.domain.description || internalPageT("toolSelection.noDescription", "No description available.")}
                    </p>
                    <div className="mt-3 flex items-center justify-between gap-3 text-[11px] text-[var(--color-on-surface-variant)]">
                      <span>
                        {entry.domain.hasMutationTools
                          ? internalPageT("toolSelection.hasMutation", "Contains mutation tools")
                          : internalPageT("toolSelection.noMutation", "Read-only only")}
                      </span>
                      <span>
                        {internalPageT("toolSelection.toolsCount", "{{count}} tools", {
                          count: entry.visibleTools.length,
                        })}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          <div className="self-start md:sticky md:top-0">
            <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
              {activeEntry ? (
                <>
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-base font-semibold text-[var(--color-on-surface)]">
                        {activeEntry.domain.name}
                      </p>
                      <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
                        {activeEntry.domain.description
                          || internalPageT("toolSelection.noDescription", "No description available.")}
                      </p>
                    </div>

                    <div className="flex flex-wrap gap-[12px]">
                      <Button
                        disabled={busy || allActiveToolsSelected}
                        size="sm"
                        variant="secondary"
                        onClick={() => {
                          onSetToolsChecked(activeDomainTools.map((tool) => tool.toolCode), true);
                        }}
                      >
                        {internalPageT("toolSelection.selectVisible", "Select all")}
                      </Button>
                      <Button
                        disabled={busy || !hasAnyActiveToolSelected}
                        size="sm"
                        variant="secondary"
                        onClick={() => {
                          onSetToolsChecked(activeDomainTools.map((tool) => tool.toolCode), false);
                        }}
                      >
                        {internalPageT("toolSelection.clearVisible", "Clear")}
                      </Button>
                    </div>
                  </div>

                  <ul className="tron-scrollbar mt-4 max-h-[36rem] space-y-2 overflow-x-hidden overflow-y-auto overscroll-y-contain pr-1">
                    {activeDomainTools.map((tool) => {
                      const checked = selectedToolSet.has(tool.toolCode);
                      return (
                        <li
                          className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-3"
                          key={tool.toolCode}
                        >
                          <label className="flex items-start gap-3">
                            <input
                              checked={checked}
                              className="mt-1 h-4 w-4 shrink-0 accent-[var(--color-primary)]"
                              disabled={busy}
                              type="checkbox"
                              onChange={(event) => {
                                onToggleTool(tool.toolCode, event.target.checked);
                              }}
                            />
                            <div className="min-w-0">
                              <div className="flex flex-wrap items-center gap-[12px]">
                                <span className="text-sm font-medium text-[var(--color-on-surface)]">
                                  {tool.name}
                                </span>
                                <span className="rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">
                                  {t(`common:toolOperationKind.${tool.operationKind}`, { defaultValue: tool.operationKind })}
                                </span>
                              </div>
                              <p className="mt-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">
                                {tool.toolCode}
                              </p>
                              <p className="mt-2 text-[13px] text-[var(--color-on-surface-variant)]">
                                {tool.description
                                  || internalPageT("toolSelection.noDescription", "No description available.")}
                              </p>
                            </div>
                          </label>
                        </li>
                      );
                    })}
                  </ul>
                </>
              ) : (
                <p className="text-sm text-[var(--color-on-surface-variant)]">
                  {internalPageT("toolSelection.emptyPanel", "Select a domain to review its tools.")}
                </p>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
