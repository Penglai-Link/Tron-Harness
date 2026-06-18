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
  return (
    <div>
      <SurfaceCard>
        <div className="grid gap-3 md:grid-cols-[minmax(260px,1fr)_180px_auto] md:items-center">
          <div className="relative min-w-0">
            <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
            <TextInput
              className="w-full pl-9"
              placeholder="搜索语料库、编码或描述..."
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
                {status === "All" ? "全部状态" : formatRagStatusLabel(status)}
              </option>
            ))}
          </SelectInput>
          <Button className="justify-self-start md:justify-self-end" size="sm" variant="secondary" onClick={onCreateCorpus}>
            <Sparkles size={17} aria-hidden="true" />
            新建语料库
          </Button>
        </div>
      </SurfaceCard>

      <SurfaceCard className="mt-5 p-0">
        <DataTable
          title="语料库列表"
          rows={corpusPagination.visibleItems}
          rowKey={(corpus) => corpus.code}
          onRowClick={onEditCorpus}
          pagination={<TablePagination itemLabel="语料库" {...corpusPagination.paginationProps} />}
          toolbar={<span className="text-[13px] text-[var(--color-on-surface-variant)]">当前显示 {filteredCount} 个语料库</span>}
          columns={[
            {
              id: "corpus",
              header: "语料库",
              render: (corpus) => (
                <div>
                  <p className="font-medium">{corpus.name}</p>
                  <p className="text-[13px] text-[var(--color-on-surface-variant)]">{corpus.code}</p>
                </div>
              ),
            },
            {
              id: "description",
              header: "描述",
              render: (corpus) => (
                <p className="line-clamp-3 text-[13px] text-[var(--color-on-surface-variant)]">
                  {corpus.description || "暂无描述"}
                </p>
              ),
            },
            {
              id: "status",
              header: "状态",
              render: (corpus) => <Badge tone={statusTone(corpus.status)}>{formatRagStatusLabel(corpus.status)}</Badge>,
            },
          ]}
        />
      </SurfaceCard>
    </div>
  );
}
