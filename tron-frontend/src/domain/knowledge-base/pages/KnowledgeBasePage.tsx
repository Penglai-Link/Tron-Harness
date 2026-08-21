import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  BookCopy,
  Database,
  FolderSync,
  Search,
  Sparkles,
} from "lucide-react";

import { listKnowledgeBases } from "@/domain/knowledge-base/mock/data";
import type { KnowledgeBaseRecord } from "@/domain/knowledge-base/model/KnowledgeBaseModel";
import { useRagCorpora } from "@/domain/rag/hooks/useRagCorpora";
import type { CorpusRecord } from "@/domain/rag/model/RagModel";
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
import { Button } from "@/components/ui/button";
import { SelectInput } from "@/shared/ui/Forms";
import { SelectItem } from "@/components/ui/select";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

const statusFilters = ["All", "Active", "Indexing", "Draft", "Disabled", "Reserved", "Warning"] as const;

function statusTone(status: KnowledgeBaseRecord["status"]) {
  switch (status) {
    case "Active":
      return "success";
    case "Indexing":
      return "info";
    case "Draft":
    case "Reserved":
    case "Warning":
      return "warning";
    case "Disabled":
      return "error";
    default:
      return "outline";
  }
}

function mapRagCorpusStatus(status: CorpusRecord["status"]): KnowledgeBaseRecord["status"] {
  switch (status) {
    case "Indexing":
      return "Indexing";
    case "Disabled":
      return "Disabled";
    case "Reserved":
      return "Reserved";
    case "Warning":
      return "Warning";
    default:
      return "Active";
  }
}

function mapRagCorpusToKnowledgeBase(corpus: CorpusRecord): KnowledgeBaseRecord {
  const ingestProfile = corpus.ingestProfile.trim();
  const retrievalProfile = corpus.retrievalProfile.trim();
  const strategySummary = corpus.strategies.join(" + ");

  return {
    chunkProfile: ingestProfile ? `Ingest profile: ${ingestProfile}` : strategySummary || "Not configured",
    documentCount: null,
    embeddingFootprint: corpus.vectorCoverage > 0
      ? `${corpus.vectorCoverage}% vector capability`
      : "Not configured",
    freshness: corpus.lastIndexed && corpus.lastIndexed !== "API" ? corpus.lastIndexed : "Not reported",
    id: corpus.code,
    name: corpus.name || corpus.code,
    ownerWorkspace: corpus.scope === "management" ? "RAG management" : corpus.scope || "RAG management",
    retrievalProfile: retrievalProfile || "Not configured",
    sourceCount: null,
    status: mapRagCorpusStatus(corpus.status),
    syncLagMinutes: null,
  };
}

export function KnowledgeBasePage() {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`knowledgeBase.${key}`, { defaultValue, ...options })
  );
  const openRightSidebar = useOpenRightSidebar();
  const dataSource = getServiceDataSource("rag");
  const {
    corpora,
    error: ragError,
    loading: ragLoading,
    refresh: refreshCorpora,
  } = useRagCorpora();
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<(typeof statusFilters)[number]>("All");
  const [selectedId, setSelectedId] = useState("");
  const [notice, setNotice] = useState("");
  const knowledgeBases = useMemo(
    () => dataSource === "api" ? corpora.map(mapRagCorpusToKnowledgeBase) : listKnowledgeBases(),
    [corpora, dataSource],
  );
  const filteredKnowledgeBases = useMemo(
    () => knowledgeBases.filter((knowledgeBase) => {
      const normalizedQuery = query.trim().toLowerCase();
      const matchesQuery = !normalizedQuery
        || knowledgeBase.name.toLowerCase().includes(normalizedQuery)
        || knowledgeBase.id.toLowerCase().includes(normalizedQuery)
        || knowledgeBase.ownerWorkspace.toLowerCase().includes(normalizedQuery)
        || knowledgeBase.retrievalProfile.toLowerCase().includes(normalizedQuery);
      const matchesStatus = statusFilter === "All" || knowledgeBase.status === statusFilter;
      return matchesQuery && matchesStatus;
    }),
    [knowledgeBases, query, statusFilter],
  );
  const pagination = useTablePagination(filteredKnowledgeBases);
  const selectedKnowledgeBase = knowledgeBases.find((knowledgeBase) => knowledgeBase.id === selectedId);
  const activeCount = knowledgeBases.filter((knowledgeBase) => knowledgeBase.status === "Active").length;
  const documentCount = knowledgeBases.reduce((total, knowledgeBase) => total + (knowledgeBase.documentCount ?? 0), 0);
  const sourceCount = knowledgeBases.reduce((total, knowledgeBase) => total + (knowledgeBase.sourceCount ?? 0), 0);
  const hasDocumentCounts = knowledgeBases.some((knowledgeBase) => knowledgeBase.documentCount !== null);
  const hasSourceCounts = knowledgeBases.some((knowledgeBase) => knowledgeBase.sourceCount !== null);
  const indexingCount = knowledgeBases.filter((knowledgeBase) => knowledgeBase.status === "Indexing").length;
  const rightPanel = useMemo(() => {
    if (!selectedKnowledgeBase) return null;
    return (
      <InspectorPanel title={pageT("inspector.title", "Selected KnowledgeBase")}>
        <div>
          <p className="text-2xl font-semibold">{selectedKnowledgeBase.name}</p>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{selectedKnowledgeBase.ownerWorkspace}</p>
        </div>

        <div className="grid gap-3 rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-low)] p-4 text-sm">
          <InspectorRow label={pageT("fields.retrievalProfile", "Retrieval Profile")} value={selectedKnowledgeBase.retrievalProfile} />
          <InspectorRow label={pageT("fields.chunkProfile", "Chunk Profile")} value={selectedKnowledgeBase.chunkProfile} />
          <InspectorRow
            label={dataSource === "api" ? pageT("fields.vectorCoverage", "Vector Coverage") : pageT("fields.embeddingFootprint", "Embedding Footprint")}
            value={selectedKnowledgeBase.embeddingFootprint}
          />
          <InspectorRow
            label={pageT("fields.syncLag", "Sync Lag")}
            value={selectedKnowledgeBase.syncLagMinutes === null
              ? pageT("values.notReported", "Not reported")
              : selectedKnowledgeBase.syncLagMinutes === 999
                ? "Draft"
                : `${selectedKnowledgeBase.syncLagMinutes} min`}
          />
        </div>

        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("inspector.operationalNotes", "Operational Notes")}</p>
          <div className="mt-3 space-y-2 text-sm text-[var(--color-on-surface-variant)]">
            <p>{dataSource === "api"
              ? pageT("inspector.apiNote", "This view is backed by the RAG corpus management API. Index counts and freshness metrics are shown only when the API reports them.")
              : pageT("inspector.mockNote", "Use this catalog for traditional RAG corpora that need predictable chunking, metadata filters, and controlled refresh windows.")}</p>
            <p>{pageT("inspector.crossDomainNote", "KnowledgeGraph stays focused on entity relationships. Resources remains the broader asset inventory for files, connectors, and raw sources.")}</p>
          </div>
        </div>

        <div className="mt-auto flex gap-2">
          <Button size="lg" className="flex-1" variant="outline" onClick={() => {
            if (dataSource === "api") {
              navigate(`/rag/corpora/${encodeURIComponent(selectedKnowledgeBase.id)}/edit`);
              return;
            }
            setNotice(pageT("messages.policyOpened", "Ingestion policy opened for {{name}}.", { name: selectedKnowledgeBase.name }));
          }}>
            {pageT("actions.reviewPolicy", "Review Policy")}
          </Button>
          <Button size="lg" className="flex-1" onClick={() => {
            if (dataSource === "api") {
              refreshCorpora();
              setNotice(pageT("messages.refreshedFromApi", "Refreshed {{name}} from the RAG API.", { name: selectedKnowledgeBase.name }));
              return;
            }
            setNotice(pageT("messages.syncStarted", "Sync job started for {{name}}.", { name: selectedKnowledgeBase.name }));
          }}>
            {dataSource === "api" ? pageT("actions.refresh", "Refresh") : pageT("actions.reindex", "Reindex Now")}
          </Button>
        </div>
      </InspectorPanel>
    );
  }, [dataSource, navigate, refreshCorpora, selectedKnowledgeBase]);

  useConsoleShellConfig({ rightPanel });

  function selectKnowledgeBase(knowledgeBase: KnowledgeBaseRecord) {
    setSelectedId(knowledgeBase.id);
    openRightSidebar();
  }

  function handleRefresh() {
    if (dataSource === "api") {
      refreshCorpora();
              setNotice(pageT("messages.catalogRefreshedFromApi", "Knowledge bases refreshed from the RAG API."));
      return;
    }
    setNotice(pageT("messages.catalogRefreshed", "Knowledge base catalog refreshed."));
  }

  function handleCreate() {
    if (dataSource === "api") {
      navigate("/rag/corpora/new");
      return;
    }
    setNotice(pageT("messages.createOpened", "Create KnowledgeBase drawer opened."));
  }

  return (
    <>
      <PageHeader
        title={pageT("title", "Knowledge Base")}
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={handleRefresh}>
              {dataSource === "api" ? pageT("actions.refresh", "Refresh") : pageT("actions.syncEmbeddings", "Sync Embeddings")}
            </Button>
            <Button size="lg" onClick={handleCreate}>
              {pageT("actions.new", "New")}
            </Button>
          </>
        )}
      />

      {dataSource === "api" && ragError ? (
        <p className="mt-4 rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error)]/10 p-3 text-sm text-[var(--color-error)]">
          {ragError.message}
        </p>
      ) : null}

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          icon={<BookCopy size={18} aria-hidden="true" />}
          label={pageT("metrics.knowledgeBases", "Knowledge Bases")}
          value={String(knowledgeBases.length)}
          trend={pageT("metrics.activeIndexing", "{{active}} active, {{indexing}} indexing", { active: activeCount, indexing: indexingCount })}
        />
        <MetricCard
          icon={<Database size={18} aria-hidden="true" />}
          label={pageT("metrics.indexedDocuments", "Indexed Documents")}
          value={hasDocumentCounts ? documentCount.toLocaleString() : "—"}
          trend={hasSourceCounts
            ? pageT("metrics.configuredSources", "{{count}} configured sources", { count: sourceCount })
            : dataSource === "api"
              ? pageT("metrics.documentCountsNotReported", "Document counts are not reported by the RAG API")
              : pageT("metrics.noUpstreamSources", "No upstream sources reported")}
        />
        <MetricCard
          icon={<FolderSync size={18} aria-hidden="true" />}
          label={pageT("metrics.freshnessSla", "Freshness SLA")}
          value={dataSource === "api" ? "—" : "< 20m"}
          trend={dataSource === "api"
            ? pageT("metrics.freshnessNotReported", "Freshness metrics are not reported by the RAG API")
            : pageT("metrics.freshnessBudget", "Index pipelines stay inside the retrieval freshness budget")}
        />
        <MetricCard
          icon={<Sparkles size={18} aria-hidden="true" />}
          label={pageT("metrics.defaultRetrieval", "Default Retrieval")}
          value={dataSource === "api"
            ? knowledgeBases.some((knowledgeBase) => knowledgeBase.retrievalProfile !== "Not configured") ? pageT("values.configured", "Configured") : "—"
            : "Hybrid"}
          trend={dataSource === "api"
            ? pageT("metrics.configuredProfiles", "Configured retrieval profiles from the RAG API")
            : pageT("metrics.hybridRetrieval", "Vector search plus lexical reranking for traditional RAG")}
        />
      </div>

      <SurfaceCard className="mt-5">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <div className="relative min-w-[280px] flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
              <TextInput
                placeholder={pageT("filters.searchPlaceholder", "Search knowledge bases, workspaces, or retrieval profiles...")}
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
                  {status === "All" ? pageT("filters.allStatuses", "All statuses") : pageT(`status.${status}`, status)}
                </SelectItem>
              ))}
            </SelectInput>
          </div>
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {pageT("description", "Traditional RAG libraries with ingestion, chunking, retrieval profiles, and freshness controls.")}
          </p>
        </div>
      </SurfaceCard>

      <div className="mt-5">
        <SurfaceCard className="p-0">
          <DataTable
            title={pageT("catalog.title", "KnowledgeBase Catalog")}
            rows={pagination.visibleItems}
            rowKey={(knowledgeBase) => knowledgeBase.id}
            onRowClick={selectKnowledgeBase}
            rowClassName={(knowledgeBase) => (
              knowledgeBase.id === selectedKnowledgeBase?.id ? "bg-[var(--color-surface-container-low)]" : undefined
            )}
            toolbar={(
              <span className="text-[13px] text-[var(--color-on-surface-variant)]">
                {pageT("catalog.visibleLibraries", "{{count}} visible libraries", { count: filteredKnowledgeBases.length })}
              </span>
            )}
            columns={[
              {
                id: "name",
                header: pageT("columns.knowledgeBase", "Knowledge Base"),
                render: (knowledgeBase) => (
                  <div>
                    <p className="font-medium">{knowledgeBase.name}</p>
                    <p className="text-[13px] text-[var(--color-on-surface-variant)]">{knowledgeBase.ownerWorkspace}</p>
                  </div>
                ),
              },
              {
                id: "corpus",
                header: pageT("columns.corpus", "Corpus"),
                render: (knowledgeBase) => (
                  <div>
                    <p>{knowledgeBase.documentCount === null ? "—" : `${knowledgeBase.documentCount.toLocaleString()} ${pageT("values.docs", "docs")}`}</p>
                    <p className="text-[13px] text-[var(--color-on-surface-variant)]">
                      {knowledgeBase.sourceCount === null ? "—" : `${knowledgeBase.sourceCount} ${pageT("values.sources", "sources")}`}
                    </p>
                  </div>
                ),
              },
              {
                id: "chunking",
                header: pageT("columns.chunking", "Chunking"),
                render: (knowledgeBase) => knowledgeBase.chunkProfile,
              },
              {
                id: "retrieval",
                header: pageT("columns.retrieval", "Retrieval"),
                render: (knowledgeBase) => knowledgeBase.retrievalProfile,
              },
              {
                id: "freshness",
                header: pageT("columns.freshness", "Freshness"),
                render: (knowledgeBase) => knowledgeBase.freshness,
              },
              {
                id: "status",
                header: pageT("columns.status", "Status"),
                render: (knowledgeBase) => <Badge tone={statusTone(knowledgeBase.status)}>{knowledgeBase.status}</Badge>,
              },
            ]}
            emptyState={ragLoading
              ? pageT("empty.loading", "Loading corpora from the RAG API...")
              : ragError
                ? pageT("empty.loadFailed", "RAG corpora could not be loaded.")
                : pageT("empty.filtered", "No knowledge bases match the current filters.")}
            pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "knowledge bases")} {...pagination.paginationProps} />}
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
