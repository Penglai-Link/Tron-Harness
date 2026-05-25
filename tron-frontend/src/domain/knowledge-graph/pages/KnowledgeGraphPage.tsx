import { useState } from "react";
import {
  Database,
  Maximize2,
  Minus,
  Network,
  Plus,
  Search,
  X,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  Field,
  SelectInput,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import { useKnowledgeGraphByDomain } from "@/domain/knowledge-graph/hooks";
import type { GraphEntity } from "@/domain/knowledge-graph/model";

function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function GraphPage({ domainFiltering = false }: { domainFiltering?: boolean }) {
  const [domain, setDomain] = useState("customer-support");
  const [query, setQuery] = useState("");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [zoom, setZoom] = useState(100);
  const graph = useKnowledgeGraphByDomain(domain);
  const visibleEntities = graph.entities.filter((entity) => entity.label.toLowerCase().includes(query.toLowerCase()) || entity.type.toLowerCase().includes(query.toLowerCase()));
  const [selectedId, setSelectedId] = useState("customer-0192");
  const selected = graph.entities.find((entity) => entity.id === selectedId) ?? graph.entities[0];

  return (
    <>
      <div className="grid min-h-[720px] overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white xl:grid-cols-[300px_minmax(520px,1fr)_320px]">
        <aside className="border-r border-[var(--color-outline-variant)] bg-white">
          <div className="border-b border-[var(--color-outline-variant)] p-4">
            <h1 className="text-xl font-semibold">Entities</h1>
            {domainFiltering ? (
              <Field label="Domain">
                <SelectInput value={domain} onChange={(event) => setDomain(event.target.value)}>
                  <option value="customer-support">Domain: Customer Support</option>
                  <option value="supply-chain">Domain: Supply Chain</option>
                  <option value="hr">Domain: HR</option>
                </SelectInput>
              </Field>
            ) : null}
            <div className="mt-3 flex h-10 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] px-3">
              <Search size={16} aria-hidden="true" />
              <input className="min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder={domainFiltering ? `Search ${graph.domainLabel}...` : "Filter nodes..."} value={query} onChange={(event) => setQuery(event.target.value)} />
            </div>
          </div>
          <div className="space-y-1 p-3">
            {visibleEntities.map((entity) => (
              <button
                className={cn("flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-[var(--color-hover-surface)]", selected.id === entity.id ? "bg-[var(--color-surface-container-high)]" : "")}
                key={entity.id}
                type="button"
                onClick={() => {
                  setSelectedId(entity.id);
                  setInspectorOpen(true);
                }}
              >
                <span className="flex size-8 items-center justify-center rounded-full bg-[var(--color-surface-container)]">
                  <Database size={15} aria-hidden="true" />
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium">{entity.label}</span>
                  <span className="block truncate text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{entity.type}</span>
                </span>
              </button>
            ))}
          </div>
        </aside>
        <GraphCanvas entities={graph.entities} selected={selected} setSelectedId={setSelectedId} setInspectorOpen={setInspectorOpen} setZoom={setZoom} zoom={zoom} />
        {inspectorOpen ? <GraphInspector selected={selected} onClose={() => setInspectorOpen(false)} /> : null}
      </div>
    </>
  );
}

function GraphCanvas({ entities, selected, setInspectorOpen, setSelectedId, setZoom, zoom }: { entities: GraphEntity[]; selected: GraphEntity; setInspectorOpen: (open: boolean) => void; setSelectedId: (id: string) => void; setZoom: React.Dispatch<React.SetStateAction<number>>; zoom: number }) {
  return (
    <section className="relative min-h-[720px] overflow-hidden bg-[var(--color-surface-container-low)]">
      <div className="absolute right-4 top-4 z-10 flex overflow-hidden rounded-lg border border-[var(--color-outline-variant)] bg-white">
        <Button size="icon" variant="ghost" onClick={() => setZoom((current) => current + 10)}><Plus size={16} /></Button>
        <Button size="icon" variant="ghost" onClick={() => setZoom((current) => Math.max(60, current - 10))}><Minus size={16} /></Button>
        <Button size="icon" variant="ghost" onClick={() => setZoom(100)}><Maximize2 size={16} /></Button>
      </div>
      <span className="absolute left-4 top-4 rounded-full bg-white px-3 py-1 text-[12px] text-[var(--color-on-surface-variant)]">Zoom {zoom}%</span>
      <svg className="absolute inset-0 h-full w-full" aria-hidden="true">
        <path d="M 350 250 L 500 400" fill="none" stroke="#c4c7c7" strokeDasharray="4 4" strokeWidth="2" />
        <path d="M 350 250 L 200 400" fill="none" stroke="#c4c7c7" strokeWidth="2" />
        <path d="M 500 400 L 650 300" fill="none" stroke="#c4c7c7" strokeWidth="2" />
      </svg>
      {entities.map((entity) => (
        <button
          className="absolute z-10 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center"
          style={{ left: entity.x, top: entity.y, transform: `translate(-50%, -50%) scale(${zoom / 100})` }}
          key={entity.id}
          type="button"
          onClick={() => {
            setSelectedId(entity.id);
            setInspectorOpen(true);
          }}
        >
          <span className={cn("flex rounded-full shadow-sm", selected.id === entity.id ? "size-14 bg-[var(--color-primary)] text-white ring-4 ring-[var(--color-accent-container)]" : "size-12 border border-[var(--color-outline-variant)] bg-white")}>
            <Network className="m-auto" size={20} aria-hidden="true" />
          </span>
          <span className="mt-2 whitespace-nowrap rounded-lg border border-[var(--color-outline-variant)] bg-white px-2 py-1 text-sm shadow-sm">{entity.label}</span>
        </button>
      ))}
    </section>
  );
}

function GraphInspector({ onClose, selected }: { onClose: () => void; selected: GraphEntity }) {
  const [notice, setNotice] = useState("");

  return (
    <aside className="border-l border-[var(--color-outline-variant)] bg-white">
      <div className="flex items-start justify-between border-b border-[var(--color-outline-variant)] p-4">
        <div>
          <h2 className="text-xl font-semibold">{selected.label}</h2>
          <p className="text-[12px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{selected.type} Node</p>
        </div>
        <Button size="icon" variant="ghost" onClick={onClose}><X size={16} /></Button>
      </div>
      <div className="space-y-5 p-4">
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Properties</h3>
          <div className="overflow-hidden rounded-lg border border-[var(--color-outline-variant)]">
            {Object.entries(selected.properties).map(([key, value]) => (
              <div className="flex justify-between gap-3 border-b border-[var(--color-outline-variant)] px-3 py-2 text-sm last:border-0" key={key}>
                <span className="text-[var(--color-on-surface-variant)]">{key}</span>
                <span className="text-right font-medium">{value}</span>
              </div>
            ))}
          </div>
        </section>
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Relationships ({selected.relationships.length})</h3>
          <div className="space-y-2">
            {selected.relationships.map((relationship) => (
              <div className="rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={`${relationship.label}-${relationship.target}`}>
                <p className="text-[var(--color-on-surface-variant)]">{relationship.label}</p>
                <p className="font-medium">{relationship.target}</p>
              </div>
            ))}
          </div>
        </section>
        <Button className="w-full" onClick={() => setNotice(`Editing ${selected.label}`)}>Edit Entity</Button>
        <Notice value={notice} />
      </div>
    </aside>
  );
}

export function KnowledgeGraphPage() {
  return <GraphPage />;
}
