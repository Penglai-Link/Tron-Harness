import {
  Download,
  Maximize2,
  Minus,
  Network,
  Plus,
} from "lucide-react";

import type { GraphFacet } from "@/domain/knowledge-graph/model";
import { SelectItem } from "@/components/ui/select";
import {
  Button,
  Field,
  SearchField,
  SelectInput,
} from "@/shared";
import { cn } from "@/shared/lib/utils";

interface GraphToolbarProps {
  activeLabels: string[];
  facets: GraphFacet[];
  namespace: string;
  namespaces: string[];
  onActiveLabelsChange: (labels: string[]) => void;
  onNamespaceChange: (namespace: string) => void;
  onQueryChange: (query: string) => void;
  onZoomChange: (delta: number) => void;
  query: string;
  relationship: string;
  relationships: string[];
  setRelationship: (relationship: string) => void;
  workspaceCode: string;
}

export function GraphToolbar({
  activeLabels,
  facets,
  namespace,
  namespaces,
  onActiveLabelsChange,
  onNamespaceChange,
  onQueryChange,
  onZoomChange,
  query,
  relationship,
  relationships,
  setRelationship,
  workspaceCode,
}: GraphToolbarProps) {
  function toggleLabel(label: string) {
    onActiveLabelsChange(
      activeLabels.includes(label)
        ? activeLabels.filter((item) => item !== label)
        : [...activeLabels, label],
    );
  }

  return (
    <div className="space-y-4 border-b border-[var(--color-outline-variant)] p-4">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Scope</span>
        <span className="rounded-full bg-[var(--color-surface-container-high)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-on-surface)]">{namespace}</span>
        <span className="rounded-full bg-[var(--color-surface-container-high)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-on-surface)]">{workspaceCode}</span>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Traversal</span>
        {relationships.slice(0, 5).map((item) => (
          <button
            className={cn(
              "rounded-full border px-2.5 py-1 font-mono text-[11px] font-semibold transition-colors",
              relationship === item
                ? "border-[var(--color-accent)] bg-[var(--color-accent-container)] text-[var(--color-on-accent-container)]"
                : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]",
            )}
            key={item}
            type="button"
            aria-pressed={relationship === item}
            onClick={() => setRelationship(relationship === item ? "all" : item)}
          >
            {item}
          </button>
        ))}
      </div>

      <div className="grid gap-3 xl:grid-cols-[minmax(260px,1fr)_190px_170px_170px_auto]">
        <SearchField
          ariaLabel="Search graph nodes"
          className="min-w-0"
          placeholder="Search node_code, label, property..."
          value={query}
          onChange={onQueryChange}
        />
        <Field label="namespace">
          <SelectInput
            aria-label="Graph namespace"
            className="h-9"
            value={namespace}
            onValueChange={(event) => onNamespaceChange(event ?? namespace)}
          >
            {namespaces.map((item) => (
              <SelectItem key={item} value={item}>{item}</SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field label="workspace">
          <SelectInput aria-label="Graph workspace scope" className="h-9" value={workspaceCode}>
            <SelectItem value={workspaceCode}>{workspaceCode}</SelectItem>
          </SelectInput>
        </Field>
        <Field label="relationship">
          <SelectInput
            aria-label="Filter relationship"
            className="h-9"
            value={relationship}
            onValueChange={(event) => setRelationship(event ?? "all")}
          >
            <SelectItem value="all">All</SelectItem>
            {relationships.map((item) => (
              <SelectItem key={item} value={item}>{item}</SelectItem>
            ))}
          </SelectInput>
        </Field>
        <div className="flex items-end gap-1">
          <Button size="icon" variant="ghost" type="button" aria-label="Zoom in" onClick={() => onZoomChange(10)}>
            <Plus size={16} aria-hidden="true" />
          </Button>
          <Button size="icon" variant="ghost" type="button" aria-label="Zoom out" onClick={() => onZoomChange(-10)}>
            <Minus size={16} aria-hidden="true" />
          </Button>
          <Button size="icon" variant="ghost" type="button" aria-label="Fit graph">
            <Maximize2 size={16} aria-hidden="true" />
          </Button>
          <Button size="icon" variant="ghost" type="button" aria-label="Expand traversal">
            <Network size={16} aria-hidden="true" />
          </Button>
          <Button size="icon" variant="ghost" type="button" aria-label="Export graph">
            <Download size={16} aria-hidden="true" />
          </Button>
        </div>
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
                  : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
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
  );
}
