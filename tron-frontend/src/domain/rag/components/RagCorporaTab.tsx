import {
  Search,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  DataTable,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";

import type { CorpusRecord } from "@/domain/rag/model/RagModel";
import {
  formatRagStatusLabel,
  statusTone,
} from "@/domain/rag/components/RagShared";

export function RagCorporaTab({
  corpusPagination,
  filteredCount,
  query,
  statusFilter,
  onCreateCorpus,
  onEditCorpus,
  onQueryChange,
  onStatusFilterChange,
}: {
  corpusPagination: ReturnType<typeof useTablePagination<CorpusRecord>>;
  filteredCount: number;
  query: string;
  statusFilter: string;
  onCreateCorpus: () => void;
  onEditCorpus: (corpus: CorpusRecord) => void;
  onQueryChange: (value: string) => void;
  onStatusFilterChange: (value: string) => void;
}) {
  const { t } = useTranslation(["pages"]);
  const { formatNumber } = useLocaleFormatters();
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.corpora.${key}`, { defaultValue, ...options })
  );

  return (
    <div>
      <SurfaceCard>
        <div className="grid gap-3 md:grid-cols-[minmax(260px,1fr)_180px_auto] md:items-center">
          <div className="relative min-w-0">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
            <TextInput
              className="w-full pl-9"
              placeholder={pageT("searchPlaceholder", "Search corpora, codes, or descriptions...")}
              type="search"
              value={query}
              onChange={(event) => onQueryChange(event.target.value)}
            />
          </div>
          <SelectInput
            className="w-full"
            value={statusFilter}
            onChange={(event) => onStatusFilterChange(event.target.value)}
          >
            {["All", "Active", "Indexing", "Warning", "Reserved"].map((status) => (
              <option key={status} value={status}>
                {status === "All" ? pageT("filters.allStatuses", "All statuses") : formatRagStatusLabel(status, t)}
              </option>
            ))}
          </SelectInput>
          <Button className="justify-self-start md:justify-self-end" size="sm" variant="secondary" onClick={onCreateCorpus}>
            <Sparkles size={17} aria-hidden="true" />
            {pageT("actions.newCorpus", "New corpus")}
          </Button>
        </div>
      </SurfaceCard>

      <SurfaceCard className="mt-5 p-0">
        <DataTable
          title={pageT("table.title", "Corpus list")}
          rows={corpusPagination.visibleItems}
          rowKey={(corpus) => corpus.code}
          onRowClick={onEditCorpus}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "corpora")} {...corpusPagination.paginationProps} />}
          toolbar={<span className="text-[13px] text-[var(--color-on-surface-variant)]">{pageT("table.visible", "{{count}} corpora visible", { count: formatNumber(filteredCount) })}</span>}
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
              id: "status",
              header: pageT("table.columns.status", "Status"),
              render: (corpus) => <Badge tone={statusTone(corpus.status)}>{formatRagStatusLabel(corpus.status, t)}</Badge>,
            },
          ]}
        />
      </SurfaceCard>
    </div>
  );
}
