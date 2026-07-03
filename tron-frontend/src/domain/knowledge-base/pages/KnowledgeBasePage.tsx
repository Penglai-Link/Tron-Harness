import { useMemo, useState } from "react";
import {
  BookCopy,
  Database,
  FolderSync,
  Search,
  Sparkles,
} from "lucide-react";

import { listKnowledgeBases } from "@/domain/knowledge-base/mock/data";
import type { KnowledgeBaseRecord } from "@/domain/knowledge-base/model/KnowledgeBaseModel";
import {
  DataTable,
  InspectorPanel,
  MetricCard,
  PageHeader,
  SurfaceCard,
  TablePagination,
  TextInput,
  useTablePagination,
  useConsoleShellConfig,
  useOpenRightSidebar,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import { SelectInput } from "@/shared/ui/Forms";
import { SelectItem } from "@/components/ui/select";

const statusFilters = ["All", "Active", "Indexing", "Draft"] as const;

function statusTone(status: KnowledgeBaseRecord["status"]) {
  switch (status) {
    case "Active":
      return "success";
    case "Indexing":
      return "info";
    case "Draft":
      return "warning";
    default:
      return "outline";
  }
}

export function KnowledgeBasePage() {
  const openRightSidebar = useOpenRightSidebar();
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<(typeof statusFilters)[number]>("All");
  const [selectedId, setSelectedId] = useState("kb-001");
  const [notice, setNotice] = useState("");
  const knowledgeBases = useMemo(() => listKnowledgeBases(), []);
  const filteredKnowledgeBases = useMemo(
    () => knowledgeBases.filter((knowledgeBase) => {
      const normalizedQuery = query.trim().toLowerCase();
      const matchesQuery = !normalizedQuery
        || knowledgeBase.name.toLowerCase().includes(normalizedQuery)
        || knowledgeBase.ownerWorkspace.toLowerCase().includes(normalizedQuery)
        || knowledgeBase.retrievalProfile.toLowerCase().includes(normalizedQuery);
      const matchesStatus = statusFilter === "All" || knowledgeBase.status === statusFilter;
      return matchesQuery && matchesStatus;
    }),
    [knowledgeBases, query, statusFilter],
  );
  const pagination = useTablePagination(filteredKnowledgeBases);
  const selectedKnowledgeBase = knowledgeBases.find((knowledgeBase) => knowledgeBase.id === selectedId) ?? knowledgeBases[0];
  const activeCount = knowledgeBases.filter((knowledgeBase) => knowledgeBase.status === "Active").length;
  const documentCount = knowledgeBases.reduce((total, knowledgeBase) => total + knowledgeBase.documentCount, 0);
  const sourceCount = knowledgeBases.reduce((total, knowledgeBase) => total + knowledgeBase.sourceCount, 0);
  const indexingCount = knowledgeBases.filter((knowledgeBase) => knowledgeBase.status === "Indexing").length;
  const rightPanel = useMemo(() => (
    <InspectorPanel title="Selected KnowledgeBase">
      <div>
        <p className="text-2xl font-semibold">{selectedKnowledgeBase.name}</p>
        <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{selectedKnowledgeBase.ownerWorkspace}</p>
      </div>

      <div className="grid gap-3 rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-low)] p-4 text-sm">
        <InspectorRow label="Retrieval Profile" value={selectedKnowledgeBase.retrievalProfile} />
        <InspectorRow label="Chunk Profile" value={selectedKnowledgeBase.chunkProfile} />
        <InspectorRow label="Embedding Footprint" value={selectedKnowledgeBase.embeddingFootprint} />
        <InspectorRow
          label="Sync Lag"
          value={selectedKnowledgeBase.syncLagMinutes === 999 ? "Draft" : `${selectedKnowledgeBase.syncLagMinutes} min`}
        />
      </div>

      <div>
        <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Operational Notes</p>
        <div className="mt-3 space-y-2 text-sm text-[var(--color-on-surface-variant)]">
          <p>Use this catalog for traditional RAG corpora that need predictable chunking, metadata filters, and controlled refresh windows.</p>
          <p>KnowledgeGraph stays focused on entity relationships. Resources remains the broader asset inventory for files, connectors, and raw sources.</p>
        </div>
      </div>

      <div className="mt-auto flex gap-2">
        <Button className="flex-1" variant="secondary" onClick={() => setNotice(`Ingestion policy opened for ${selectedKnowledgeBase.name}.`)}>
          Review Policy
        </Button>
        <Button className="flex-1" onClick={() => setNotice(`Sync job started for ${selectedKnowledgeBase.name}.`)}>
          Reindex Now
        </Button>
      </div>
    </InspectorPanel>
  ), [selectedKnowledgeBase]);

  useConsoleShellConfig({ rightPanel });

  function selectKnowledgeBase(knowledgeBase: KnowledgeBaseRecord) {
    setSelectedId(knowledgeBase.id);
    openRightSidebar();
  }

  return (
    <>
      <PageHeader
        title="Knowledge Base"
        actions={(
          <>
            <Button variant="secondary" onClick={() => setNotice("Embedding sync queued for all active knowledge bases.")}>
              <FolderSync size={16} aria-hidden="true" />
              Sync Embeddings
            </Button>
            <Button onClick={() => setNotice("Create KnowledgeBase drawer opened.")}>
              <Sparkles size={16} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          icon={<BookCopy size={18} aria-hidden="true" />}
          label="Knowledge Bases"
          value={String(knowledgeBases.length)}
          trend={`${activeCount} active, ${indexingCount} indexing`}
        />
        <MetricCard
          icon={<Database size={18} aria-hidden="true" />}
          label="Indexed Documents"
          value={documentCount.toLocaleString()}
          trend={`${sourceCount} upstream corpora`}
        />
        <MetricCard
          icon={<FolderSync size={18} aria-hidden="true" />}
          label="Freshness SLA"
          value="< 20m"
          trend="Index pipelines stay inside the retrieval freshness budget"
        />
        <MetricCard
          icon={<Sparkles size={18} aria-hidden="true" />}
          label="Default Retrieval"
          value="Hybrid"
          trend="Vector search plus lexical reranking for traditional RAG"
        />
      </div>

      <SurfaceCard className="mt-5">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <div className="relative min-w-[280px] flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
              <TextInput
                className="h-10 pl-9"
                placeholder="Search knowledge bases, workspaces, or retrieval profiles..."
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </div>
            <SelectInput
              className="min-w-[180px]"
              value={statusFilter}
              onValueChange={(event) => setStatusFilter(event! as (typeof statusFilters)[number])}
            >
              {statusFilters.map((status) => (
                <SelectItem key={status} value={status}>
                  {status === "All" ? "All statuses" : status}
                </SelectItem>
              ))}
            </SelectInput>
          </div>
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            Traditional RAG libraries with ingestion, chunking, retrieval profiles, and freshness controls.
          </p>
        </div>
      </SurfaceCard>

      <div className="mt-5">
        <SurfaceCard className="p-0">
          <DataTable
            title="KnowledgeBase Catalog"
            rows={pagination.visibleItems}
            rowKey={(knowledgeBase) => knowledgeBase.id}
            onRowClick={selectKnowledgeBase}
            rowClassName={(knowledgeBase) => (
              knowledgeBase.id === selectedKnowledgeBase.id ? "bg-[var(--color-surface-container-low)]" : undefined
            )}
            toolbar={(
              <span className="text-[13px] text-[var(--color-on-surface-variant)]">
                {filteredKnowledgeBases.length} visible libraries
              </span>
            )}
            columns={[
              {
                id: "name",
                header: "Knowledge Base",
                render: (knowledgeBase) => (
                  <div>
                    <p className="font-medium">{knowledgeBase.name}</p>
                    <p className="text-[13px] text-[var(--color-on-surface-variant)]">{knowledgeBase.ownerWorkspace}</p>
                  </div>
                ),
              },
              {
                id: "corpus",
                header: "Corpus",
                render: (knowledgeBase) => (
                  <div>
                    <p>{knowledgeBase.documentCount.toLocaleString()} docs</p>
                    <p className="text-[13px] text-[var(--color-on-surface-variant)]">{knowledgeBase.sourceCount} sources</p>
                  </div>
                ),
              },
              {
                id: "chunking",
                header: "Chunking",
                render: (knowledgeBase) => knowledgeBase.chunkProfile,
              },
              {
                id: "retrieval",
                header: "Retrieval",
                render: (knowledgeBase) => knowledgeBase.retrievalProfile,
              },
              {
                id: "freshness",
                header: "Freshness",
                render: (knowledgeBase) => knowledgeBase.freshness,
              },
              {
                id: "status",
                header: "Status",
                render: (knowledgeBase) => <Badge tone={statusTone(knowledgeBase.status)}>{knowledgeBase.status}</Badge>,
              },
            ]}
            pagination={<TablePagination itemLabel="knowledge bases" {...pagination.paginationProps} />}
          />
        </SurfaceCard>
      </div>

      <p className="sr-only" aria-live="polite">{notice}</p>
    </>
  );
}

function InspectorRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="font-medium text-[var(--color-on-surface)]">{value}</span>
    </div>
  );
}
