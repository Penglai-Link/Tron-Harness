import {
  Search,
  Sparkles,
} from "lucide-react";

import {
  DataTable,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";

import type { CorpusRecord } from "@/domain/rag/model/RagModel";
import {
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
  return (
    <div>
      <SurfaceCard>
        <div className="grid gap-3 md:grid-cols-[minmax(260px,1fr)_180px_auto] md:items-center">
          <div className="relative min-w-0">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
            <TextInput
              className="w-full pl-9"
              placeholder="Search corpus, code, or description..."
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
                {status === "All" ? "All statuses" : status}
              </option>
            ))}
          </SelectInput>
          <Button className="justify-self-start md:justify-self-end" size="sm" variant="secondary" onClick={onCreateCorpus}>
            <Sparkles size={17} aria-hidden="true" />
            New Corpus
          </Button>
        </div>
      </SurfaceCard>

      <SurfaceCard className="mt-5 p-0">
        <DataTable
          title="Corpus Registry"
          rows={corpusPagination.visibleItems}
          rowKey={(corpus) => corpus.code}
          onRowClick={onEditCorpus}
          pagination={<TablePagination itemLabel="corpora" {...corpusPagination.paginationProps} />}
          toolbar={<span className="text-[13px] text-[var(--color-on-surface-variant)]">{filteredCount} visible corpora</span>}
          columns={[
            {
              id: "corpus",
              header: "Corpus",
              render: (corpus) => (
                <div>
                  <p className="font-medium">{corpus.name}</p>
                  <p className="text-[13px] text-[var(--color-on-surface-variant)]">{corpus.code}</p>
                </div>
              ),
            },
            {
              id: "description",
              header: "Description",
              render: (corpus) => (
                <p className="line-clamp-3 text-[13px] text-[var(--color-on-surface-variant)]">
                  {corpus.description || "No description"}
                </p>
              ),
            },
            {
              id: "status",
              header: "Status",
              render: (corpus) => <Badge tone={statusTone(corpus.status)}>{corpus.status}</Badge>,
            },
          ]}
        />
      </SurfaceCard>
    </div>
  );
}
