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
  onToggleTool: (toolCode: string, checked: boolean) => void;
  selectedToolCodes: string[];
  toolDomains: ToolCatalogDomainRecord[];
  tools: PlatformToolRecord[];
  allowRemovingUnavailable?: boolean;
  onSetToolsChecked?: (codes: string[], checked: boolean) => void;
}

interface DomainEntry {
  domain: ToolCatalogDomainRecord;
  categoryCode: string;
  categoryName: string;
  selectedCount: number;
  visibleTools: PlatformToolRecord[];
}

function normalizeSearchText(value: string) {
  return value.trim().toLowerCase();
}

export function InternalAgentToolSelector({
  className,
  busy = false,
  onToggleTool,
  selectedToolCodes,
  toolDomains,
  tools,
  allowRemovingUnavailable = false,
  onSetToolsChecked,
}: InternalAgentToolSelectorProps) {
  const { t, i18n } = useTranslation(["pages", "common"]);
  const [categoryFilter, setCategoryFilter] = useState("all");
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

  const categoryEntries = useMemo(() => {
    const grouped = new Map<string, { code: string; name: string; count: number }>();
    for (const tool of tools) {
      if (!tool.category) continue;
      const code = tool.category.code;
      const current = grouped.get(code) ?? { code, name: tool.category.names[i18n.resolvedLanguage ?? i18n.language] ?? tool.category.names["en-US"] ?? code, count: 0 };
      current.count++;
      grouped.set(code, current);
    }
    return [...grouped.values()].sort((a, b) => a.name.localeCompare(b.name));
  }, [tools, i18n.language, i18n.resolvedLanguage]);

  const domainEntries = useMemo(() => {
    const grouped = new Map<string, { categoryCode: string; categoryName: string; domainCode: string; tools: PlatformToolRecord[] }>();
    for (const tool of tools) {
      const categoryCode = tool.category?.code ?? "uncategorized";
      if (categoryFilter === "readonly" ? tool.operationKind !== "query" : categoryFilter !== "all" && categoryCode !== categoryFilter) continue;
      const categoryName = categoryEntries.find((item) => item.code === categoryCode)?.name ?? categoryCode;
      const key = `${categoryCode}:${tool.domainCode}`;
      const entry = grouped.get(key) ?? { categoryCode, categoryName, domainCode: tool.domainCode, tools: [] };
      entry.tools.push(tool);
      grouped.set(key, entry);
    }
    return [...grouped.values()].flatMap(({categoryCode, categoryName, domainCode, tools: domainTools}): DomainEntry[] => {
      const domain = toolDomains.find((item) => item.domainCode === domainCode) ?? { domainCode, name: domainCode, description: "", sampleTools: [], toolCount: domainTools.length, hasMutationTools: false };
      const matchesDomain = [categoryCode, categoryName, domain.domainCode, domain.name, domain.description, ...domain.sampleTools].join(" ").toLowerCase().includes(debouncedToolSearch);
      const visibleTools = (matchesDomain ? domainTools : domainTools.filter((tool) => [tool.toolCode, tool.toolName, tool.name, tool.description, tool.operationKind, t(`common:toolOperationKind.${tool.operationKind}`, {defaultValue: tool.operationKind})].join(" ").toLowerCase().includes(debouncedToolSearch))).sort((a, b) => a.toolCode.localeCompare(b.toolCode));
      if (!visibleTools.length) return [];
      return [{ domain, categoryCode, categoryName, visibleTools, selectedCount: domainTools.filter((tool) => tool.assignable !== false && selectedToolSet.has(tool.toolCode)).length }];
    });
  }, [tools, categoryFilter, categoryEntries, toolDomains, debouncedToolSearch, selectedToolSet, t]);

  return (
    <div className={cn("mt-5 flex h-full min-h-0 flex-col gap-3", className)}>
      <div className="flex shrink-0 flex-wrap gap-2" role="group" aria-label={t("pages:settingsTools.categories.label", { defaultValue: "Tool categories" })}>
        {[{code: "all", name: t("pages:settingsTools.categories.all", { defaultValue: "All tools" }), count: tools.length}, ...categoryEntries, {code: "readonly", name: t("pages:settingsTools.categories.readonly", {defaultValue: "Read-only tools"}), count: tools.filter((tool) => tool.operationKind === "query").length}].map((category) => <Button key={category.code} type="button" size="sm" variant={categoryFilter === category.code ? "default" : "outline"} aria-pressed={categoryFilter === category.code} onClick={() => setCategoryFilter(category.code)}>{category.name} ({category.count})</Button>)}
      </div>
      <Field className="shrink-0" label={internalPageT("toolSelection.domainsTitle", "Domains")} hintAriaLabel={internalPageT("toolSelection.helpAria", "Explain tool selection")} hint={<p>{internalPageT("toolSelection.expandHint", "Expand a domain to choose its tools.")}</p>}>
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
      <div className="tron-scrollbar min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-y-contain">
      {domainEntries.length === 0 ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {internalPageT(
            "toolSelection.empty",
            "No tools match the search query."
          )}
        </p>
      ) : (
        <ul className="divide-y divide-[var(--color-outline-variant)] overflow-hidden rounded-xl border border-[var(--color-outline-variant)]">
          {domainEntries.map((entry) => {
            const { domain, categoryCode, visibleTools, selectedCount } = entry;
            const groupKey = `${categoryCode}:${domain.domainCode}`;
            const expanded = expandedDomainCodes.has(groupKey);
            const headingId = `${domainIdPrefix}-${groupKey}-heading`;
            const panelId = `${domainIdPrefix}-${groupKey}-tools`;

            return (
              <li key={groupKey}>
                <button
                  id={headingId}
                  type="button"
                  aria-expanded={expanded}
                  aria-controls={expanded ? panelId : undefined}
                  className="flex w-full items-center gap-3 bg-[var(--color-surface-container-lowest)] px-4 py-3 text-left transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-[var(--color-primary)]"
                  onClick={() =>
                    setExpandedDomainCodes((current) => {
                      const next = new Set(current);
                      if (next.has(groupKey))
                        next.delete(groupKey);
                      else next.add(groupKey);
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
                    {onSetToolsChecked && <div className="flex flex-wrap gap-2">
                      <Button type="button" size="sm" variant="outline" disabled={busy || !visibleTools.some((tool) => tool.assignable !== false)} onClick={() => onSetToolsChecked(visibleTools.filter((tool) => tool.assignable !== false).map((tool) => tool.toolCode), true)}>
                        {internalPageT("assistantToolPolicy.selectVisible", "Select shown tools")}
                      </Button>
                      <Button type="button" size="sm" variant="outline" disabled={busy || !visibleTools.some((tool) => selectedToolSet.has(tool.toolCode))} onClick={() => onSetToolsChecked(visibleTools.map((tool) => tool.toolCode), false)}>
                        {internalPageT("assistantToolPolicy.clearVisible", "Clear shown tools")}
                      </Button>
                    </div>}
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
                              checked={selectedToolSet.has(tool.toolCode) && (tool.assignable !== false || allowRemovingUnavailable)}
                              className="mt-1 h-4 w-4 shrink-0 accent-[var(--color-primary)]"
                              disabled={busy || (tool.assignable === false && !(allowRemovingUnavailable && selectedToolSet.has(tool.toolCode)))}
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
                                {tool.toolName}
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
    </div>
  );
}
