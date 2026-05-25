import { useState } from "react";
import {
  Database,
  Plus,
  RefreshCw,
  Search,
  Upload,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SegmentedTabs,
  SurfaceCard,
  TablePagination,
  useTablePagination,
} from "@/shared";

import { useMemoryFragmentList } from "@/domain/memory/hooks";
import type { MemoryFragment } from "@/domain/memory/model";
import {
  defaultMemoryFilters,
  memoryStrategyTabs,
} from "@/domain/memory/constants";

function SmallStatus({ children, tone = "neutral" }: { children: React.ReactNode; tone?: "neutral" | "accent" | "success" | "warning" | "error" }) {
  return <Badge tone={tone}>{children}</Badge>;
}

function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function MemoryRepository({ details = false }: { details?: boolean }) {
  const fragments = useMemoryFragmentList();
  const [strategy, setStrategy] = useState("hybrid");
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<string[]>([...defaultMemoryFilters]);
  const [selectedId, setSelectedId] = useState("mem-1");
  const [notice, setNotice] = useState("");
  const visible = fragments.filter((fragment) => fragment.content.toLowerCase().includes(query.toLowerCase()) || fragment.namespace.toLowerCase().includes(query.toLowerCase()));
  const memoryPagination = useTablePagination(visible);
  const selected = fragments.find((fragment) => fragment.id === selectedId) ?? fragments[0];

  const table = (
    <DataTable
      title="Memory Fragments"
      rows={memoryPagination.visibleItems}
      rowKey={(row) => row.id}
      toolbar={<span className="text-[13px] text-[var(--color-on-surface-variant)]">{visible.length} matching fragments</span>}
      columns={[
        { id: "content", header: "Content Snippet", render: (row) => <button className="max-w-[380px] truncate text-left" type="button" onClick={() => setSelectedId(row.id)}>{row.content}</button> },
        { id: "namespace", header: "Namespace", render: (row) => row.namespace },
        { id: "entity", header: "Entity", render: (row) => <span className="font-mono">{row.entity}</span> },
        { id: "source", header: "Source", render: (row) => row.source },
        { id: "confidence", header: "Confidence", align: "right", render: (row) => row.confidence.toFixed(2) },
        { id: "created", header: "Created", render: (row) => row.created },
        { id: "inheritance", header: "Inheritance", render: (row) => <SmallStatus tone={row.inheritance === "Override" ? "accent" : "neutral"}>{row.inheritance}</SmallStatus> },
      ]}
      pagination={<TablePagination itemLabel="fragments" {...memoryPagination.paginationProps} />}
    />
  );

  return (
    <>
      <PageHeader
        title="Memory Repository"
        actions={<><Button variant="secondary" onClick={() => setNotice("Import data drawer opened")}><Upload size={16} />Import Data</Button><Button onClick={() => setNotice("Index sync queued")}><RefreshCw size={16} />Sync Indices</Button></>}
      />
      <Notice value={notice} />
      <SurfaceCard className="mb-4">
        <div className="flex flex-col gap-3 md:flex-row md:items-center">
          <div className="flex h-10 flex-1 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-white px-3">
            <Search size={16} />
            <input className="min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder="Query memory vectors via natural language or exact match..." value={query} onChange={(event) => setQuery(event.target.value)} />
          </div>
          <SegmentedTabs value={strategy} onValueChange={setStrategy} items={memoryStrategyTabs} />
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Active Filters:</span>
          {filters.map((filter) => (
            <span className="inline-flex items-center gap-1 rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-high)] px-3 py-1 text-sm" key={filter}>
              {filter}
              <button type="button" onClick={() => setFilters((current) => current.filter((item) => item !== filter))}><X size={13} /></button>
            </span>
          ))}
          <Button size="sm" variant="ghost" onClick={() => setFilters((current) => [...current, `Strategy: ${strategy}`])}><Plus size={14} />Add Filter</Button>
        </div>
      </SurfaceCard>
      {details ? <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_400px]">{table}<MemoryInspector fragment={selected} /></div> : table}
    </>
  );
}

function MemoryInspector({ fragment }: { fragment: MemoryFragment }) {
  const [status, setStatus] = useState("Selected");
  return (
    <SurfaceCard className="flex flex-col p-0">
      <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] p-5">
        <h2 className="text-xl font-semibold">Memory Details</h2>
        <Button size="icon" variant="ghost" onClick={() => setStatus("Inspector closed")}><X size={16} /></Button>
      </div>
      <div className="space-y-6 p-5">
        <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Content Snippet</h3><div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm leading-6">{fragment.content} This includes removing pleasantries and preserving information density.</div></section>
        <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Metadata</h3><div className="grid grid-cols-2 gap-4 rounded-lg border border-[var(--color-outline-variant)] p-4 text-sm"><Meta label="Entity" value={fragment.entity} /><Meta label="Namespace" value={fragment.namespace} /><Meta label="Confidence" value={fragment.confidence.toFixed(2)} /><Meta label="Inheritance" value={fragment.inheritance} /><Meta label="Created" value={`${fragment.created} 14:22:45 UTC`} /></div></section>
        <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Origin</h3><div className="flex items-center gap-3 rounded-lg border border-[var(--color-outline-variant)] p-4"><Database size={18} /><span><span className="block text-sm font-medium">{fragment.source}</span><span className="block text-[12px] text-[var(--color-on-surface-variant)]">Primary Environment</span></span></div></section>
        <p className="text-[13px] text-[var(--color-on-surface-variant)]">{status}</p>
      </div>
      <div className="mt-auto flex gap-2 border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
        <Button className="flex-1" variant="secondary" onClick={() => setStatus("Edit mode enabled")}>Edit Fragment</Button>
        <Button className="flex-1" onClick={() => setStatus("Fragment invalidated")}>Invalidate</Button>
      </div>
    </SurfaceCard>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return <div><p className="text-[11px] text-[var(--color-on-surface-variant)]">{label}</p><p className="mt-1 font-medium">{value}</p></div>;
}

export function MemoryDetailsPage() {
  return <MemoryRepository details />;
}
