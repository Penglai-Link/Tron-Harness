import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  GraphFacet,
  GraphLayoutMode,
  GraphNodeView,
} from "@/domain/knowledge-graph/model";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { SelectInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { MultiSelectDropdown, type MultiSelectOption } from "@/components/multi-select-dropdown";
import { SearchField } from "@/components/search-field";

interface GraphToolbarProps {
  activeLabels: string[];
  facets: GraphFacet[];
  layoutMode: GraphLayoutMode;
  locatorNodes: GraphNodeView[];
  locatorQuery: string;
  nodeLabelProperties: string[];
  nodeLabelProperty: string;
  namespace: string;
  namespaces: string[];
  onActiveLabelsChange: (labels: string[]) => void;
  onLocatorQueryChange: (query: string) => void;
  onLocateNode: (nodeCode: string) => void;
  onLayoutModeChange: (layoutMode: GraphLayoutMode) => void;
  onNamespaceChange: (namespace: string) => void;
  onNodeLabelPropertyChange: (property: string) => void;
  onRefresh: () => void;
  onSelectedRelationshipsChange: (relationships: string[]) => void;
  onToggleFullscreen: () => void;
  onZoomChange: (delta: number) => void;
  refreshing: boolean;
  relationships: string[];
  selectedRelationships: string[];
  fullscreenActive: boolean;
}

const graphLayoutOptions: GraphLayoutMode[] = ["Radial", "Smart", "Force", "Hierarchy", "Grid"];

interface ToolbarFieldProps {
  children: ReactNode;
  className: string;
  hint: ReactNode;
  label: string;
}

function ToolbarField({ children, className, hint, label }: ToolbarFieldProps) {
  const { t } = useTranslation(["pages"]);

  return (
    <div className={`flex min-w-0 flex-col items-stretch gap-[12px] ${className}`}>
      <div className="inline-flex self-start items-center gap-[12px] whitespace-nowrap font-medium text-base/4.5 text-foreground sm:text-sm/4">
        <span>{label}</span>
        <InfoTooltip ariaLabel={t("pages:knowledgeGraph.toolbar.fieldHelpAria", {
          defaultValue: "{{label}} field help",
          label,
        })}>
          {hint}
        </InfoTooltip>
      </div>
      {children}
    </div>
  );
}

function nodeTitle(node: GraphNodeView) {
  const title = String(node.properties.title ?? "").trim();
  return title || node.nodeCode;
}

function compactNodeCode(nodeCode: string) {
  if (nodeCode.length <= 24) return nodeCode;
  return `${nodeCode.slice(0, 14)}...${nodeCode.slice(-7)}`;
}

function nodeMatchesLocatorQuery(node: GraphNodeView, query: string) {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return false;

  return [
    node.nodeCode,
    nodeTitle(node),
  ].some((value) => value.toLowerCase().includes(normalizedQuery));
}

export function GraphToolbar({
  activeLabels,
  facets,
  layoutMode,
  locatorNodes,
  locatorQuery,
  nodeLabelProperties,
  nodeLabelProperty,
  namespace,
  namespaces,
  onActiveLabelsChange,
  onLocatorQueryChange,
  onLocateNode,
  onLayoutModeChange,
  onNamespaceChange,
  onNodeLabelPropertyChange,
  onRefresh,
  onSelectedRelationshipsChange,
  onToggleFullscreen,
  onZoomChange,
  refreshing,
  relationships,
  selectedRelationships,
  fullscreenActive,
}: GraphToolbarProps) {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:knowledgeGraph.${key}`, { defaultValue, ...options })
  );
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [locatorOpen, setLocatorOpen] = useState(false);
  const activeFilterCount = activeLabels.length + selectedRelationships.length;
  const locatorMatches = useMemo(
    () => locatorNodes
      .filter((node) => nodeMatchesLocatorQuery(node, locatorQuery))
      .slice(0, 8),
    [locatorNodes, locatorQuery],
  );
  const showLocatorPopup = locatorOpen && locatorQuery.trim().length > 0;
  const activeLabelFacets = useMemo(() => {
    const facetByValue = new Map(facets.map((facet) => [facet.value, facet]));
    return activeLabels.map((label) => facetByValue.get(label) ?? {
      count: 0,
      label,
      value: label,
    });
  },
    [activeLabels, facets],
  );
  const labelOptions = useMemo<MultiSelectOption[]>(
    () => facets.map((facet) => ({
      description: pageT("toolbar.nodeCount", "{{count}} nodes", { count: facet.count }),
      label: facet.label,
      value: facet.value,
    })),
    [facets, t],
  );
  const relationshipOptions = useMemo<MultiSelectOption[]>(() => {
    const values = new Set(relationships.filter((item) => item.trim().length > 0));
    for (const relationship of selectedRelationships) {
      if (relationship.trim().length > 0) {
        values.add(relationship);
      }
    }
    return Array.from(values)
      .sort()
      .map((relationship) => ({
        label: relationship,
        value: relationship,
      }));
  }, [relationships, selectedRelationships]);
  const showLabelFilter = labelOptions.length > 0 || activeLabels.length > 0;
  const showRelationshipFilter = relationshipOptions.length > 0 || selectedRelationships.length > 0;

  function toggleLabel(label: string) {
    onActiveLabelsChange(
      activeLabels.includes(label)
        ? activeLabels.filter((item) => item !== label)
        : [...activeLabels, label],
    );
  }

  function toggleRelationship(relationship: string) {
    onSelectedRelationshipsChange(
      selectedRelationships.includes(relationship)
        ? selectedRelationships.filter((item) => item !== relationship)
        : [...selectedRelationships, relationship],
    );
  }

  function clearLabelFilters() {
    onActiveLabelsChange([]);
  }

  function clearRelationshipFilters() {
    onSelectedRelationshipsChange([]);
  }

  function locateNode(node: GraphNodeView) {
    onLocateNode(node.nodeCode);
    onLocatorQueryChange(nodeTitle(node));
    setLocatorOpen(false);
  }

  return (
    <div className="relative z-30 space-y-3 overflow-visible border-b border-[var(--color-outline-variant)] p-3">
      <div className="overflow-visible pb-1">
        <div className="grid min-w-0 grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-[minmax(0,420px)_minmax(0,1fr)_minmax(0,1fr)]">
          <ToolbarField
            className="w-full min-w-0 sm:col-span-2 lg:col-span-1 lg:w-[420px]"
            label={pageT("toolbar.locate.label", "Locate node")}
            hint={pageT("toolbar.locate.hint", "Type a node code or title, then choose a match to select and center it on the canvas.")}
          >
            <div className="relative w-full">
              <SearchField
                ariaLabel={pageT("toolbar.locate.aria", "Locate graph node")}
                inputClassName="h-10 pl-3 sm:h-9"
                placeholder={pageT("toolbar.locate.placeholder", "Locate node by code or title...")}
                value={locatorQuery}
                onBlur={() => {
                  window.setTimeout(() => setLocatorOpen(false), 120);
                }}
                onChange={(value) => {
                  onLocatorQueryChange(value);
                  setLocatorOpen(true);
                }}
                onFocus={() => setLocatorOpen(true)}
              />
              {showLocatorPopup ? (
                <div
                  className="absolute left-0 right-0 top-full z-40 mt-2 max-h-72 overflow-y-auto rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-1 shadow-xl"
                  role="listbox"
                  aria-label={pageT("toolbar.locate.matchesAria", "Matching graph nodes")}
                >
                  {locatorMatches.length ? locatorMatches.map((node) => {
                    const primaryLabel = node.labels[0] ?? "GraphNode";
                    return (
                      <button
                        className="flex w-full min-w-0 items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
                        key={node.nodeCode}
                        type="button"
                        role="option"
                        aria-label={`${nodeTitle(node)} ${node.nodeCode} ${primaryLabel}`}
                        aria-selected="false"
                        onMouseDown={(event) => event.preventDefault()}
                        onClick={() => locateNode(node)}
                      >
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-[13px] font-semibold text-[var(--color-on-surface)]">{nodeTitle(node)}</span>
                          <span className="mt-1 flex min-w-0 items-center gap-1.5">
                            <span className="inline-flex h-5 shrink-0 items-center rounded-full bg-[var(--color-surface-container-high)] px-2 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
                              {primaryLabel}
                            </span>
                            <span className="inline-flex h-5 min-w-0 max-w-[12rem] shrink items-center overflow-hidden rounded-full bg-[var(--color-surface-container-high)] px-2 font-mono text-[11px] whitespace-nowrap text-[var(--color-on-surface-variant)]" title={node.nodeCode}>
                              {compactNodeCode(node.nodeCode)}
                            </span>
                          </span>
                        </span>
                      </button>
                    );
                  }) : (
                    <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">
                      {pageT("toolbar.locate.empty", "No matching nodes.")}
                    </p>
                  )}
                </div>
              ) : null}
            </div>
          </ToolbarField>
          <ToolbarField
            className="w-full min-w-0"
            label={pageT("toolbar.namespace.label", "Namespace")}
            hint={pageT("toolbar.namespace.hint", "Choose which graph namespace to browse.")}
          >
            <SelectInput
              aria-label={pageT("toolbar.namespace.aria", "Graph namespace")}
              className="w-full"
              value={namespace}
              disabled={namespaces.length === 0}
              onValueChange={(event) => onNamespaceChange(event ?? namespace)}
            >
              {namespaces.map((item) => (
                <SelectItem key={item} value={item}>{item}</SelectItem>
              ))}
            </SelectInput>
          </ToolbarField>
          <ToolbarField
            className="w-full min-w-0"
            label={pageT("toolbar.displayField.label", "Display field")}
            hint={pageT("toolbar.displayField.hint", "Choose the node property shown beside enlarged graph nodes.")}
          >
            <SelectInput
              aria-label={pageT("toolbar.displayField.aria", "Graph node label property")}
              className="w-full"
              value={nodeLabelProperty}
              disabled={nodeLabelProperties.length === 0}
              onValueChange={(event) => onNodeLabelPropertyChange(event ?? nodeLabelProperty)}
            >
              {nodeLabelProperties.map((item) => (
                <SelectItem key={item} value={item}>{item}</SelectItem>
              ))}
            </SelectInput>
          </ToolbarField>
          <div className="flex min-w-0 items-end gap-1 sm:col-span-2 sm:gap-[12px] lg:col-span-3">
            <ToolbarField
              className="min-w-0 flex-1 sm:w-[220px] sm:flex-none lg:w-[148px]"
              label={pageT("toolbar.layout.label", "Layout")}
              hint={pageT("toolbar.layout.hint", "Choose how nodes are arranged on the graph canvas.")}
            >
              <SelectInput
                aria-label={pageT("toolbar.layout.aria", "Graph layout")}
                className="w-full min-w-0"
                value={layoutMode}
                onValueChange={(event) => onLayoutModeChange((event ?? "Radial") as GraphLayoutMode)}
              >
                {graphLayoutOptions.map((item) => (
                  <SelectItem key={item} value={item}>{pageT(`toolbar.layout.options.${item}`, item)}</SelectItem>
                ))}
              </SelectInput>
            </ToolbarField>
            <Button
              className="h-9 shrink-0 px-2 sm:px-3"
              size="sm"
              variant={filtersOpen || activeFilterCount ? "secondary" : "ghost"}
              type="button"
              aria-expanded={filtersOpen}
              aria-label={pageT("toolbar.filters.toggleAria", "Toggle graph filters")}
              onClick={() => setFiltersOpen((current) => !current)}
            >

              <span className="hidden sm:inline">{pageT("toolbar.filters.label", "Filters")}</span>
              {activeFilterCount ? (
                <span className="rounded-full bg-[var(--color-primary)] px-1.5 font-mono text-[10px] text-[var(--color-on-primary)]">
                  {activeFilterCount}
                </span>
              ) : null}
            </Button>
            <div className="ml-auto flex shrink-0 items-center gap-1 sm:gap-[12px]">
              <Button className="w-auto sm:w-auto" size="sm" variant="ghost" type="button" aria-label={pageT("toolbar.zoomIn", "Zoom in")} title={pageT("toolbar.zoomIn", "Zoom in")} onClick={() => onZoomChange(10)}>{pageT("toolbar.zoomIn", "Zoom in")}</Button>
              <Button className="w-auto sm:w-auto" size="sm" variant="ghost" type="button" aria-label={pageT("toolbar.zoomOut", "Zoom out")} title={pageT("toolbar.zoomOut", "Zoom out")} onClick={() => onZoomChange(-10)}>{pageT("toolbar.zoomOut", "Zoom out")}</Button>
              <Button
                className="w-auto sm:w-auto"
                size="sm"
                variant={fullscreenActive ? "secondary" : "ghost"}
                type="button"
                aria-label={fullscreenActive ? pageT("toolbar.fullscreen.exitAria", "Exit graph fullscreen") : pageT("toolbar.fullscreen.openAria", "Open graph fullscreen")}
                title={fullscreenActive ? pageT("toolbar.fullscreen.exit", "Exit fullscreen") : pageT("toolbar.fullscreen.open", "Fullscreen")}
                onClick={onToggleFullscreen}
              >{fullscreenActive ? pageT("toolbar.fullscreen.exitAria", "Exit graph fullscreen") : pageT("toolbar.fullscreen.openAria", "Open graph fullscreen")}</Button>
              <Button
                className="w-auto sm:w-auto"
                size="sm"
                variant="ghost"
                type="button"
                aria-label={pageT("actions.refresh", "Refresh graph")}
                title={pageT("actions.refresh", "Refresh graph")}
                disabled={!namespace || refreshing}
                onClick={onRefresh}
              >{pageT("actions.refresh", "Refresh graph")}</Button>
            </div>
          </div>
        </div>
      </div>

      {!filtersOpen && activeFilterCount ? (
        <div className="flex flex-wrap items-center gap-[12px] px-0.5 text-[12px] text-[var(--color-on-surface-variant)]">
          <span className="font-medium">{pageT("toolbar.filters.active", "Active filters")}</span>
          {selectedRelationships.map((relationship) => (
            <button
              className="inline-flex h-7 items-center gap-1 rounded-full border border-[var(--color-accent)] bg-[var(--color-accent-container)] px-2 font-mono text-[11px] font-semibold text-[var(--color-on-accent-container)]"
              key={relationship}
              type="button"
              onClick={() => toggleRelationship(relationship)}
            >
              {relationship}

            </button>
          ))}
          {activeLabelFacets.map((facet) => (
            <button
              className="inline-flex h-7 items-center gap-1 rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2 text-[12px] font-medium text-[var(--color-on-surface)]"
              key={facet.value}
              type="button"
              onClick={() => toggleLabel(facet.value)}
            >
              {facet.label}

            </button>
          ))}
        </div>
      ) : null}

      {filtersOpen ? (
        <div className="border-t border-[var(--color-outline-variant)] pt-3">
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-[minmax(260px,360px)_minmax(260px,360px)]">
            {showRelationshipFilter ? (
              <div className="min-w-0">
                <div className="mb-1.5 flex h-6 items-center justify-between gap-[12px]">
                  <p className="text-sm font-semibold text-[var(--color-on-surface)]">
                    {pageT("toolbar.relationships.label", "Relationship")}
                  </p>
                  {selectedRelationships.length ? (
                    <Button className="h-6 px-2 text-[12px]" size="sm" variant="ghost" type="button" onClick={clearRelationshipFilters}>
                      {pageT("toolbar.filters.clear", "Clear")}
                    </Button>
                  ) : null}
                </div>
                <MultiSelectDropdown
                  ariaLabel={pageT("toolbar.relationships.aria", "Filter relationship")}
                  emptyMessage={pageT("toolbar.relationships.empty", "No relationships.")}
                  options={relationshipOptions}
                  placeholder={pageT("toolbar.relationships.placeholder", "All relationships")}
                  searchPlaceholder={pageT("toolbar.relationships.searchPlaceholder", "Search relationships...")}
                  selectedValues={selectedRelationships}
                  onToggle={toggleRelationship}
                />
              </div>
            ) : null}
            {showLabelFilter ? (
              <div className="min-w-0">
                <div className="mb-1.5 flex h-6 items-center justify-between gap-[12px]">
                  <p className="text-sm font-semibold text-[var(--color-on-surface)]">
                    {pageT("toolbar.labels.label", "Labels")}
                  </p>
                  {activeLabels.length ? (
                    <Button className="h-6 px-2 text-[12px]" size="sm" variant="ghost" type="button" onClick={clearLabelFilters}>
                      {pageT("toolbar.filters.clear", "Clear")}
                    </Button>
                  ) : null}
                </div>
                <MultiSelectDropdown
                  ariaLabel={pageT("toolbar.labels.aria", "Filter node labels")}
                  emptyMessage={pageT("toolbar.labels.empty", "No labels.")}
                  options={labelOptions}
                  placeholder={pageT("toolbar.labels.placeholder", "All labels")}
                  searchPlaceholder={pageT("toolbar.labels.searchPlaceholder", "Search labels...")}
                  selectedValues={activeLabels}
                  onToggle={toggleLabel}
                />
              </div>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
