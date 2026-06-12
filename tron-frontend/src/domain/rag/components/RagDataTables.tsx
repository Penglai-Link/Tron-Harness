import {
  DataTable,
  TablePagination,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";

import type {
  SourceRecord,
  TextUnitRecord,
  TraceRecord,
  WarningRecord,
} from "@/domain/rag/model/RagModel";
import { statusTone } from "@/domain/rag/components/RagShared";

export function SourceDataTable({ pagination, onSelectSource }: { pagination: ReturnType<typeof useTablePagination<SourceRecord>>; onSelectSource: (source: SourceRecord) => void }) {
  return (
    <DataTable
      title="Sources"
      rows={pagination.visibleItems}
      rowKey={(source) => source.code}
      onRowClick={onSelectSource}
      pagination={<TablePagination itemLabel="sources" {...pagination.paginationProps} />}
      columns={[
        {
          id: "source",
          header: "Source",
          render: (source) => (
            <div>
              <p className="font-medium">{source.sourceType}</p>
              <p className="break-all text-[13px] text-[var(--color-on-surface-variant)]">{source.code}</p>
            </div>
          ),
        },
        { id: "corpus", header: "Corpus", render: (source) => source.corpus },
        { id: "owner", header: "Owner", render: (source) => source.owner },
        { id: "units", header: "Units", align: "right", render: (source) => source.textUnits.toLocaleString() },
        { id: "indexed", header: "Indexed", render: (source) => source.indexedAt },
        { id: "status", header: "Status", render: (source) => <Badge tone={statusTone(source.status)}>{source.status}</Badge> },
      ]}
    />
  );
}

export function TextUnitDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<TextUnitRecord>> }) {
  return (
    <DataTable
      title="Text Units"
      rows={pagination.visibleItems}
      rowKey={(unit) => unit.id}
      pagination={<TablePagination itemLabel="text units" {...pagination.paginationProps} />}
      columns={[
        { id: "id", header: "ID", render: (unit) => <span className="font-mono text-[13px]">{unit.id}</span> },
        { id: "source", header: "Source", render: (unit) => <span className="break-all">{unit.source}</span> },
        { id: "preview", header: "Preview", render: (unit) => <span className="text-[var(--color-on-surface-variant)]">{unit.preview}</span> },
        { id: "tokens", header: "Tokens", align: "right", render: (unit) => unit.tokens.toLocaleString() },
        { id: "status", header: "Embedding", render: (unit) => <Badge tone={statusTone(unit.embeddingStatus)}>{unit.embeddingStatus}</Badge> },
      ]}
    />
  );
}

export function WarningDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<WarningRecord>> }) {
  return (
    <DataTable
      title="Warnings"
      rows={pagination.visibleItems}
      rowKey={(warning) => `${warning.source}-${warning.stage}-${warning.age}`}
      pagination={<TablePagination itemLabel="warnings" {...pagination.paginationProps} />}
      columns={[
        { id: "severity", header: "Severity", render: (warning) => <Badge tone={statusTone(warning.severity)}>{warning.severity}</Badge> },
        { id: "source", header: "Source", render: (warning) => <span className="break-all">{warning.source}</span> },
        { id: "stage", header: "Stage", render: (warning) => warning.stage },
        { id: "message", header: "Message", render: (warning) => <span className="text-[var(--color-on-surface-variant)]">{warning.message}</span> },
        { id: "age", header: "Age", align: "right", render: (warning) => warning.age },
      ]}
    />
  );
}

export function TraceDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<TraceRecord>> }) {
  return (
    <DataTable
      title="Search Traces"
      rows={pagination.visibleItems}
      rowKey={(trace) => trace.id}
      pagination={<TablePagination itemLabel="traces" {...pagination.paginationProps} />}
      columns={[
        { id: "id", header: "Trace", render: (trace) => <span className="font-mono text-[13px]">{trace.id}</span> },
        { id: "query", header: "Query", render: (trace) => trace.query },
        { id: "profile", header: "Profile", render: (trace) => trace.profile },
        { id: "route", header: "Route", render: (trace) => trace.route },
        { id: "hits", header: "Hits", align: "right", render: (trace) => trace.hits.toLocaleString() },
        { id: "tokens", header: "Tokens", align: "right", render: (trace) => trace.tokens.toLocaleString() },
      ]}
    />
  );
}
