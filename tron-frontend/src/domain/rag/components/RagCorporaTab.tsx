import {
  useState,
} from "react";
import {
  MoreHorizontal,
  Sparkles
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import {
  DataTable,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

import type { CorpusRecord } from "@/domain/rag/model/RagModel";
import {
  formatRagStatusLabel,
  statusTone,
} from "@/domain/rag/components/RagShared";
import { SelectItem } from "@/components/ui/select";

export function RagCorporaTab({
  corpusPagination,
  query,
  statusFilter,
  onCreateCorpus,
  onDeleteCorpus,
  onDeleteIngestProfile,
  onEditCorpus,
  onTestIngest,
  onQueryChange,
  onStatusFilterChange,
}: {
  corpusPagination: ReturnType<typeof useTablePagination<CorpusRecord>>;
  query: string;
  statusFilter: string;
  onCreateCorpus: () => void;
  onDeleteCorpus: (corpus: CorpusRecord) => Promise<void> | void;
  onDeleteIngestProfile: (corpus: CorpusRecord) => Promise<void> | void;
  onEditCorpus: (corpus: CorpusRecord) => void;
  onTestIngest: (corpus: CorpusRecord) => void;
  onQueryChange: (value: string) => void;
  onStatusFilterChange: (value: string) => void;
}) {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages"]);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.corpora.${key}`, { defaultValue, ...options })
  );

  async function runRowAction(corpus: CorpusRecord, action: (target: CorpusRecord) => Promise<void> | void) {
    if (rowBusyCode === corpus.code) return;
    setRowBusyCode(corpus.code);
    try {
      await action(corpus);
    } finally {
      setRowBusyCode("");
    }
  }

  function ingestProfilePath(corpus: CorpusRecord) {
    const ingestProfileId = corpus.ingestProfile.trim();
    if (ingestProfileId) {
      return `/rag/profiles/ingest/${encodeURIComponent(ingestProfileId)}/edit?returnProfileType=ingest`;
    }
    return `/rag/profiles/ingest/new?returnProfileType=ingest&corpusId=${encodeURIComponent(corpus.code)}`;
  }

  function openIngestProfile(corpus: CorpusRecord) {
    navigate(ingestProfilePath(corpus));
  }

  return (
    <>
      <SurfaceCard className="p-0">
        <div className="border-b border-[var(--color-outline-variant)] p-4 flex items-center justify-between">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h2 className="text-base font-semibold">{pageT("table.title", "Corpus")}</h2>
          </div>
          <div className="grid gap-3 md:grid-cols-[minmax(260px,1fr)_180px_auto] md:items-center">
            <div className="relative min-w-0">
              <TextInput
                placeholder={pageT("searchPlaceholder", "Search corpora, codes, or descriptions...")}
                value={query}
                onChange={(event) => onQueryChange(event.target.value)}
              />
            </div>
            <SelectInput
              className="w-full"
              value={statusFilter}
              onValueChange={(event) => onStatusFilterChange(event!)}
            >
              {["All", "Active", "Indexing", "Warning", "Disabled"].map((status) => (
                <SelectItem key={status} value={status}>
                  {status === "All" ? pageT("filters.allStatuses", "All statuses") : formatRagStatusLabel(status, t)}
                </SelectItem>
              ))}
            </SelectInput>
            <Button size={'lg'} onClick={onCreateCorpus}>
              {pageT("actions.newCorpus", "New corpus")}
            </Button>
          </div>
        </div>

        <DataTable
          rows={corpusPagination.visibleItems}
          rowKey={(corpus) => corpus.code}
          variant="embedded"
          onRowClick={onEditCorpus}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "corpora")} {...corpusPagination.paginationProps} />}
          columns={[
            {
              id: "corpus",
              header: pageT("table.columns.corpus", "Corpus"),
              render: (corpus) => (
                <div>
                  <p className="font-medium">{corpus.name}</p>
                  <p className="text-[13px] text-[var(--color-on-surface-variant)]">{corpus.code}</p>
                </div>
              ),
            },
            {
              id: "description",
              header: pageT("table.columns.description", "Description"),
              render: (corpus) => (
                <p className="line-clamp-3 text-[13px] text-[var(--color-on-surface-variant)]">
                  {corpus.description || pageT("table.noDescription", "No description")}
                </p>
              ),
            },
            {
              id: "ingestProfile",
              header: pageT("table.columns.ingestProfile", "Ingest profile"),
              render: (corpus) => {
                const ingestProfileId = corpus.ingestProfile.trim();
                return ingestProfileId ? (
                  <p className="max-w-[260px] truncate font-mono text-[13px]" title={ingestProfileId}>
                    {ingestProfileId}
                  </p>
                ) : (
                  <span className="text-[13px] text-[var(--color-on-surface-variant)]">
                    {pageT("table.noIngestProfile", "Unbound")}
                  </span>
                );
              },
              width: "240px",
            },
            {
              id: "status",
              header: pageT("table.columns.status", "Status"),
              render: (corpus) => <Badge variant={statusTone(corpus.status)}>{formatRagStatusLabel(corpus.status, t)}</Badge>,
            },
            {
              align: "right",
              cellClassName: "w-24",
              header: pageT("table.columns.actions", "Actions"),
              headerClassName: "w-24",
              id: "actions",
              render: (corpus) => {
                const isBusy = rowBusyCode === corpus.code;
                return (
                  <div className="flex justify-end">
                    <DropdownMenu>
                      <DropdownMenuTrigger disabled={isBusy}>
                        <MoreHorizontal onClick={(event) => {
                          event.stopPropagation();
                        }} size={16} aria-hidden="true" />
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end" className="w-44">
                        <DropdownMenuItem
                          disabled={isBusy}
                          onClick={(event) => {
                            event.stopPropagation();
                            openIngestProfile(corpus);
                          }}
                        >
                          {corpus.ingestProfile.trim()
                            ? pageT("actions.editIngestProfile", "Edit ingest profile")
                            : pageT("actions.createIngestProfile", "Create ingest profile")}
                        </DropdownMenuItem>
                        {corpus.ingestProfile.trim() ? (
                          <>
                            <DropdownMenuItem
                              disabled={isBusy}
                              onClick={(event) => {
                                event.stopPropagation();
                                onTestIngest(corpus);
                              }}
                            >
                              {pageT("actions.testIngest", "Ingest test")}
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              variant="destructive"
                              disabled={isBusy}
                              onClick={(event) => {
                                event.stopPropagation();
                                void runRowAction(corpus, onDeleteIngestProfile);
                              }}
                            >
                              {pageT("actions.deleteIngestProfile", "Delete ingest profile")}
                            </DropdownMenuItem>
                          </>
                        ) : null}
                        <DropdownMenuItem
                          variant="destructive"
                          disabled={isBusy}
                          onClick={(event) => {
                            event.stopPropagation();
                            void runRowAction(corpus, onDeleteCorpus);
                          }}
                        >
                          {pageT("actions.deleteCorpus", "Delete corpus")}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                );
              },
            },
          ]}
        />
      </SurfaceCard>
    </>
  );
}
