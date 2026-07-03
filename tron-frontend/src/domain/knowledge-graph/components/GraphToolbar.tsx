import {
  Maximize2,
  Minimize2,
  Minus,
  Plus,
  RefreshCw,
  SlidersHorizontal,
  X,
} from "lucide-react";
import type { ReactNode } from "react";
import { useMemo, useState } from "react";

import type {
  GraphFacet,
  GraphLayoutMode,
  GraphNodeView,
} from "@/domain/knowledge-graph/model";
import { SelectItem } from "@/components/ui/select";
import {
  Button,
  MultiSelectDropdown,
  type MultiSelectOption,
  SearchField,
  SelectInput,
  InfoTooltip,
} from "@/shared";

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

const graphLayoutOptions: Array<{ label: string; value: GraphLayoutMode }> = [
  { label: "Radial", value: "Radial" },
  { label: "Smart", value: "Smart" },
  { label: "Force", value: "Force" },
  { label: "Hierarchy", value: "Hierarchy" },
  { label: "Grid", value: "Grid" },
];

interface ToolbarFieldProps {
  children: ReactNode;
  className: string;
  hint: ReactNode;
  label: string;
}

function ToolbarField({ children, className, hint, label }: ToolbarFieldProps) {
  return (
    <div className={`flex min-w-0 flex-col items-stretch gap-2 ${className}`}>
      <div className="inline-flex self-start items-center gap-2 font-medium text-base/4.5 text-foreground sm:text-sm/4">
        <span>{label}</span>
        <InfoTooltip ariaLabel={`${label} field help`}>
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
      description: `${facet.count} nodes`,
      label: facet.label,
      value: facet.value,
    })),
    [facets],
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
        <div className="flex min-w-[1180px] items-end gap-2">
          <ToolbarField
            className="w-[420px] shrink-0"
            label="Locate node"
            hint="Type a node code or title, then choose a match to select and center it on the canvas."
          >
            <div className="relative w-full">
              <SearchField
                ariaLabel="Locate graph node"
                inputClassName="h-10 pl-3 sm:h-9"
                placeholder="Locate node by code or title..."
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
                  aria-label="Matching graph nodes"
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
                    <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">No matching nodes.</p>
                  )}
                </div>
              ) : null}
            </div>
          </ToolbarField>
          <ToolbarField
            className="w-[160px] shrink-0"
            label="Namespace"
            hint="Choose which graph namespace to browse."
          >
            <SelectInput
              aria-label="Graph namespace"
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
            className="w-[160px] shrink-0"
            label="Display field"
            hint="Choose the node property shown beside enlarged graph nodes."
          >
            <SelectInput
              aria-label="Graph node label property"
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
          <div className="flex shrink-0 items-end gap-2">
            <ToolbarField
              className="w-[148px]"
              label="Layout"
              hint="Choose how nodes are arranged on the graph canvas."
            >
              <SelectInput
                aria-label="Graph layout"
                className="w-full"
                value={layoutMode}
                onValueChange={(event) => onLayoutModeChange((event ?? "Radial") as GraphLayoutMode)}
              >
                {graphLayoutOptions.map((item) => (
                  <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>
                ))}
              </SelectInput>
            </ToolbarField>
            <Button
              className="h-9 px-3"
              size="sm"
              variant={filtersOpen || activeFilterCount ? "secondary" : "ghost"}
              type="button"
              aria-expanded={filtersOpen}
              aria-label="Toggle graph filters"
              onClick={() => setFiltersOpen((current) => !current)}
            >
              <SlidersHorizontal size={15} aria-hidden="true" />
              Filters
              {activeFilterCount ? (
                <span className="rounded-full bg-[var(--color-primary)] px-1.5 font-mono text-[10px] text-[var(--color-on-primary)]">
                  {activeFilterCount}
                </span>
              ) : null}
            </Button>
          </div>
          <div className="ml-auto flex shrink-0 items-center gap-2">
            <Button className="size-9" size="icon" variant="ghost" type="button" aria-label="Zoom in" title="Zoom in" onClick={() => onZoomChange(10)}>
              <Plus size={16} aria-hidden="true" />
            </Button>
            <Button className="size-9" size="icon" variant="ghost" type="button" aria-label="Zoom out" title="Zoom out" onClick={() => onZoomChange(-10)}>
              <Minus size={16} aria-hidden="true" />
            </Button>
            <Button
              className="size-9"
              size="icon"
              variant={fullscreenActive ? "secondary" : "ghost"}
              type="button"
              aria-label={fullscreenActive ? "Exit graph fullscreen" : "Open graph fullscreen"}
              title={fullscreenActive ? "Exit fullscreen" : "Fullscreen"}
              onClick={onToggleFullscreen}
            >
              {fullscreenActive ? <Minimize2 size={16} aria-hidden="true" /> : <Maximize2 size={16} aria-hidden="true" />}
            </Button>
            <Button
              className="size-9"
              size="icon"
              variant="ghost"
              type="button"
              aria-label="Refresh graph"
              title="Refresh graph"
              disabled={!namespace || refreshing}
              onClick={onRefresh}
            >
              <RefreshCw className={refreshing ? "animate-spin" : undefined} size={16} aria-hidden="true" />
            </Button>
          </div>
        </div>
      </div>

      {!filtersOpen && activeFilterCount ? (
        <div className="flex flex-wrap items-center gap-2 px-0.5 text-[12px] text-[var(--color-on-surface-variant)]">
          <span className="font-medium">Active filters</span>
          {selectedRelationships.map((relationship) => (
            <button
              className="inline-flex h-7 items-center gap-1 rounded-full border border-[var(--color-accent)] bg-[var(--color-accent-container)] px-2 font-mono text-[11px] font-semibold text-[var(--color-on-accent-container)]"
              key={relationship}
              type="button"
              onClick={() => toggleRelationship(relationship)}
            >
              {relationship}
              <X size={12} aria-hidden="true" />
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
              <X size={12} aria-hidden="true" />
            </button>
          ))}
        </div>
      ) : null}

      {filtersOpen ? (
        <div className="border-t border-[var(--color-outline-variant)] pt-3">
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-[minmax(260px,360px)_minmax(260px,360px)]">
            {showRelationshipFilter ? (
              <div className="min-w-0">
                <div className="mb-1.5 flex h-6 items-center justify-between gap-2">
                  <p className="text-sm font-semibold text-[var(--color-on-surface)]">Relationship</p>
                  {selectedRelationships.length ? (
                    <Button className="h-6 px-2 text-[12px]" size="sm" variant="ghost" type="button" onClick={clearRelationshipFilters}>
                      Clear
                    </Button>
                  ) : null}
                </div>
                <MultiSelectDropdown
                  ariaLabel="Filter relationship"
                  emptyMessage="No relationships."
                  options={relationshipOptions}
                  placeholder="All relationships"
                  searchPlaceholder="Search relationships..."
                  selectedValues={selectedRelationships}
                  onToggle={toggleRelationship}
                />
              </div>
            ) : null}
            {showLabelFilter ? (
              <div className="min-w-0">
                <div className="mb-1.5 flex h-6 items-center justify-between gap-2">
                  <p className="text-sm font-semibold text-[var(--color-on-surface)]">Labels</p>
                  {activeLabels.length ? (
                    <Button className="h-6 px-2 text-[12px]" size="sm" variant="ghost" type="button" onClick={clearLabelFilters}>
                      Clear
                    </Button>
                  ) : null}
                </div>
                <MultiSelectDropdown
                  ariaLabel="Filter node labels"
                  emptyMessage="No labels."
                  options={labelOptions}
                  placeholder="All labels"
                  searchPlaceholder="Search labels..."
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
