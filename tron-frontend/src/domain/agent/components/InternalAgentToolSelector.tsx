import { DisabledResourceReason } from "./InternalAgentSkillField";
import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

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
  const domainIdPrefix = useId();
  const [expandedDomainCodes, setExpandedDomainCodes] = useState<Set<string>>(
    () => new Set()
  );
  const selectedToolSet = useMemo(
    () => new Set(selectedToolCodes),
    [selectedToolCodes]
  );

  const internalPageT = (
    key: string,
    defaultValue: string,
    options?: Record<string, unknown>
  ) => t(`pages:workspaceAgents.internal.${key}`, { defaultValue, ...options });

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
        [...items].sort((left, right) =>
          left.toolCode.localeCompare(right.toolCode)
        )
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

      const matchesDomain =
        normalizedSearch.length === 0 ||
        [
          domain.domainCode,
          domain.name,
          domain.description,
          domain.sampleTools.join(" "),
        ]
          .join(" ")
          .toLowerCase()
          .includes(normalizedSearch);

      const visibleTools = matchesDomain
        ? domainTools
        : domainTools.filter((tool) =>
            [
              tool.toolCode,
              tool.domainCode,
              tool.name,
              tool.description,
              tool.operationKind,
              t(`common:toolOperationKind.${tool.operationKind}`, {
                defaultValue: tool.operationKind,
              }),
            ]
              .join(" ")
              .toLowerCase()
              .includes(normalizedSearch)
          );

      if (!visibleTools.length) {
        return accum;
      }

      accum.push({
        domain,
        selectedCount: domainTools.filter(
          (tool) =>
            tool.assignable !== false && selectedToolSet.has(tool.toolCode)
        ).length,
        visibleTools,
      });

      return accum;
    }, []);
  }, [allToolsByDomain, debouncedToolSearch, selectedToolSet, t, toolDomains]);

  return (
    <div className={cn("mt-5 space-y-3", className)}>
      <Field label={internalPageT("toolSelection.domainsTitle", "Domains")}>
        <SearchField
          className="w-full"
          ariaLabel={internalPageT("toolSelection.searchAria", "Search tools")}
          placeholder={internalPageT(
            "toolSelection.searchPlaceholder",
            "Search tools..."
          )}
          value={toolSearch}
          onChange={setToolSearch}
        />
      </Field>
      <p className="text-xs text-[var(--color-on-surface-variant)]">
        {internalPageT(
          "toolSelection.expandHint",
          "Expand a domain to choose its tools."
        )}
      </p>
      {domainEntries.length === 0 ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {internalPageT(
            "toolSelection.empty",
            "No tools match the search query."
          )}
        </p>
      ) : (
        <ul className="tron-scrollbar max-h-[min(32rem,55dvh)] divide-y divide-[var(--color-outline-variant)] overflow-x-hidden overflow-y-auto overscroll-y-contain rounded-xl border border-[var(--color-outline-variant)]">
          {domainEntries.map((entry) => {
            const { domain, visibleTools, selectedCount } = entry;
            const expanded = expandedDomainCodes.has(domain.domainCode);
            const headingId = `${domainIdPrefix}-${domain.domainCode}-heading`;
            const panelId = `${domainIdPrefix}-${domain.domainCode}-tools`;
            const assignableTools = visibleTools.filter(
              (tool) => tool.assignable !== false
            );
            const allSelected =
              assignableTools.length > 0 &&
              assignableTools.every((tool) =>
                selectedToolSet.has(tool.toolCode)
              );
            const hasSelection = visibleTools.some((tool) =>
              selectedToolSet.has(tool.toolCode)
            );

            return (
              <li key={domain.domainCode}>
                <button
                  id={headingId}
                  type="button"
                  aria-expanded={expanded}
                  aria-controls={expanded ? panelId : undefined}
                  className="flex w-full items-center gap-3 bg-[var(--color-surface-container-lowest)] px-4 py-3 text-left transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-[var(--color-primary)]"
                  onClick={() =>
                    setExpandedDomainCodes((current) => {
                      const next = new Set(current);
                      if (next.has(domain.domainCode))
                        next.delete(domain.domainCode);
                      else next.add(domain.domainCode);
                      return next;
                    })
                  }
                >
                  <ChevronRight
                    aria-hidden="true"
                    className={cn(
                      "size-4 shrink-0 text-[var(--color-on-surface-variant)] transition-transform",
                      expanded && "rotate-90"
                    )}
                  />
                  <span className="min-w-0 flex-1 truncate text-sm font-medium text-[var(--color-on-surface)]">
                    {domain.name}
                  </span>
                  <span className="shrink-0 text-xs text-[var(--color-on-surface-variant)]">
                    {internalPageT(
                      "toolSelection.toolsCount",
                      "{{count}} tools",
                      { count: visibleTools.length }
                    )}
                  </span>
                  <span
                    className={cn(
                      "shrink-0 rounded-full px-2 py-0.5 text-xs",
                      selectedCount > 0
                        ? "bg-[var(--color-primary)]/10 text-[var(--color-primary)]"
                        : "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)]"
                    )}
                  >
                    {internalPageT(
                      "toolSelection.selectedCount",
                      "{{count}} selected",
                      { count: selectedCount }
                    )}
                  </span>
                </button>
                {expanded && (
                  <div
                    id={panelId}
                    role="region"
                    aria-labelledby={headingId}
                    className="space-y-3 bg-[var(--color-surface-container-low)] p-3 sm:pl-11"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <p className="min-w-0 flex-1 text-xs text-[var(--color-on-surface-variant)]">
                        {domain.description ||
                          internalPageT(
                            "toolSelection.noDescription",
                            "No description available."
                          )}
                      </p>
                      <div className="flex shrink-0 gap-2">
                        <Button
                          type="button"
                          disabled={
                            busy || assignableTools.length === 0 || allSelected
                          }
                          size="sm"
                          variant="secondary"
                          onClick={() =>
                            onSetToolsChecked(
                              assignableTools.map((tool) => tool.toolCode),
                              true
                            )
                          }
                        >
                          {internalPageT(
                            "toolSelection.selectVisible",
                            "Select all"
                          )}
                        </Button>
                        <Button
                          type="button"
                          disabled={busy || !hasSelection}
                          size="sm"
                          variant="secondary"
                          onClick={() =>
                            onSetToolsChecked(
                              visibleTools.map((tool) => tool.toolCode),
                              false
                            )
                          }
                        >
                          {internalPageT("toolSelection.clearVisible", "Clear")}
                        </Button>
                      </div>
                    </div>
                    <ul className="divide-y divide-[var(--color-outline-variant)] rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)]">
                      {visibleTools.map((tool) => (
                        <li key={tool.toolCode} className="p-3">
                          <DisabledResourceReason
                            reason={tool.disabledReason}
                          />
                          <label
                            className={cn(
                              "flex items-start gap-3",
                              tool.assignable === false && "opacity-60"
                            )}
                          >
                            <input
                              checked={
                                tool.assignable !== false &&
                                selectedToolSet.has(tool.toolCode)
                              }
                              className="mt-1 h-4 w-4 shrink-0 accent-[var(--color-primary)]"
                              disabled={busy || tool.assignable === false}
                              type="checkbox"
                              onChange={(event) =>
                                onToggleTool(
                                  tool.toolCode,
                                  event.target.checked
                                )
                              }
                            />
                            <div className="min-w-0">
                              <div className="flex flex-wrap items-center gap-2">
                                <span className="text-sm font-medium text-[var(--color-on-surface)]">
                                  {tool.name}
                                </span>
                                <span className="rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] text-[var(--color-on-surface-variant)]">
                                  {t(
                                    `common:toolOperationKind.${tool.operationKind}`,
                                    { defaultValue: tool.operationKind }
                                  )}
                                </span>
                              </div>
                              <p className="mt-1 break-all font-mono text-[11px] text-[var(--color-on-surface-variant)]">
                                {tool.toolCode}
                              </p>
                              <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                                {tool.description ||
                                  internalPageT(
                                    "toolSelection.noDescription",
                                    "No description available."
                                  )}
                              </p>
                            </div>
                          </label>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
