import {
  Download,
  Maximize2,
  Minimize2,
  Minus,
  Plus,
  SlidersHorizontal,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";

import type {
  GraphFacet,
  GraphLayoutMode,
  GraphMetrics,
} from "@/domain/knowledge-graph/model";
import { SelectItem } from "@/components/ui/select";
import {
  Button,
  SearchField,
  SelectInput,
} from "@/shared";
import { cn } from "@/shared/lib/utils";

interface GraphToolbarProps {
  activeLabels: string[];
  facets: GraphFacet[];
  layoutMode: GraphLayoutMode;
  metrics: GraphMetrics;
  namespace: string;
  namespaces: string[];
  onActiveLabelsChange: (labels: string[]) => void;
  onLayoutModeChange: (layoutMode: GraphLayoutMode) => void;
  onNamespaceChange: (namespace: string) => void;
  onQueryChange: (query: string) => void;
  onToggleFullscreen: () => void;
  onZoomChange: (delta: number) => void;
  query: string;
  relationship: string;
  relationships: string[];
  setRelationship: (relationship: string) => void;
  fullscreenActive: boolean;
  workspaceCode: string;
}

function numberText(value: number) {
  return new Intl.NumberFormat("en-US").format(value);
}

const graphLayoutOptions: Array<{ label: string; value: GraphLayoutMode }> = [
  { label: "Radial", value: "Radial" },
  { label: "Smart", value: "Smart" },
  { label: "Force", value: "Force" },
  { label: "Hierarchy", value: "Hierarchy" },
  { label: "Grid", value: "Grid" },
];

export function GraphToolbar({
  activeLabels,
  facets,
  layoutMode,
  metrics,
  namespace,
  namespaces,
  onActiveLabelsChange,
  onLayoutModeChange,
  onNamespaceChange,
  onQueryChange,
  onToggleFullscreen,
  onZoomChange,
  query,
  relationship,
  relationships,
  setRelationship,
  fullscreenActive,
  workspaceCode,
}: GraphToolbarProps) {
  const [filtersOpen, setFiltersOpen] = useState(false);
  const activeFilterCount = activeLabels.length + (relationship === "all" ? 0 : 1);
  const activeLabelFacets = useMemo(
    () => facets.filter((facet) => activeLabels.includes(facet.value)),
    [activeLabels, facets],
  );

  function toggleLabel(label: string) {
    onActiveLabelsChange(
      activeLabels.includes(label)
        ? activeLabels.filter((item) => item !== label)
        : [...activeLabels, label],
    );
  }

  function clearLabelFilters() {
    onActiveLabelsChange([]);
  }

  return (
    <div className="space-y-3 border-b border-[var(--color-outline-variant)] p-3">
      <div className="grid items-center gap-2 xl:grid-cols-[minmax(280px,1fr)_160px_190px_auto_auto]">
        <SearchField
          ariaLabel="Search graph nodes"
          className="min-w-0"
          placeholder="Search node_code, label, property..."
          value={query}
          onChange={onQueryChange}
        />
        <SelectInput
          aria-label="Graph namespace"
          className="h-9"
          size="sm"
          value={namespace}
          onValueChange={(event) => onNamespaceChange(event ?? namespace)}
        >
          {namespaces.map((item) => (
            <SelectItem key={item} value={item}>{item}</SelectItem>
          ))}
        </SelectInput>
        <SelectInput
          aria-label="Filter relationship"
          className="h-9"
          size="sm"
          value={relationship}
          onValueChange={(event) => setRelationship(event ?? "all")}
        >
          <SelectItem value="all">All relationships</SelectItem>
          {relationships.map((item) => (
            <SelectItem key={item} value={item}>{item}</SelectItem>
          ))}
        </SelectInput>
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex h-9 items-center rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
            {workspaceCode}
          </span>
          <SelectInput
            aria-label="Graph layout"
            className="h-9 w-[148px]"
            size="sm"
            value={layoutMode}
            onValueChange={(event) => onLayoutModeChange((event ?? "Radial") as GraphLayoutMode)}
          >
            {graphLayoutOptions.map((item) => (
              <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>
            ))}
          </SelectInput>
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
        <div className="flex justify-end gap-2">
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
          <Button className="size-9" size="icon" variant="ghost" type="button" aria-label="Export graph" title="Export graph">
            <Download size={16} aria-hidden="true" />
          </Button>
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-end gap-2 px-0.5 font-mono text-[11px] text-[var(--color-on-surface-variant)]">
        <span>{numberText(metrics.nodeCount)} nodes</span>
        <span className="text-[var(--color-outline)]">·</span>
        <span>{numberText(metrics.edgeCount)} edges</span>
        <span className="text-[var(--color-outline)]">·</span>
        <span>{numberText(metrics.namespaceCount)} namespaces</span>
      </div>

      {!filtersOpen && activeFilterCount ? (
        <div className="flex flex-wrap items-center gap-2 px-0.5 text-[12px] text-[var(--color-on-surface-variant)]">
          <span className="font-medium">Active filters</span>
          {relationship !== "all" ? (
            <button
              className="inline-flex h-7 items-center gap-1 rounded-full border border-[var(--color-accent)] bg-[var(--color-accent-container)] px-2 font-mono text-[11px] font-semibold text-[var(--color-on-accent-container)]"
              type="button"
              onClick={() => setRelationship("all")}
            >
              {relationship}
              <X size={12} aria-hidden="true" />
            </button>
          ) : null}
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
          <div className="mb-3 flex items-center justify-between gap-3">
            <p className="text-sm font-semibold text-[var(--color-on-surface)]">Node types</p>
            {activeLabels.length ? (
              <Button className="h-8 px-2" size="sm" variant="ghost" type="button" onClick={clearLabelFilters}>
                Clear
              </Button>
            ) : null}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {facets.map((facet) => {
              const active = activeLabels.includes(facet.value);
              return (
                <button
                  className={cn(
                    "inline-flex h-8 items-center gap-2 rounded-lg border px-3 text-[13px] font-medium transition-colors",
                    active
                      ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                      : "border-[var(--color-outline-variant)] bg-white text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                  )}
                  key={facet.value}
                  type="button"
                  aria-pressed={active}
                  onClick={() => toggleLabel(facet.value)}
                >
                  {facet.label}
                  <span className={cn("font-mono text-[11px]", active ? "text-[var(--color-on-primary)]/80" : "text-[var(--color-on-surface-variant)]")}>{facet.count}</span>
                </button>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
