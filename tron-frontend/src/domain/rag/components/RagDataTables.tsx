import {
  DataTable,
  TablePagination,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";
import { useTranslation } from "react-i18next";
import { Badge } from "@/shared/ui/Badge";

import type {
  SourceRecord,
  TextUnitRecord,
  TraceRecord,
  WarningRecord,
} from "@/domain/rag/model/RagModel";
import {
  formatRagStatusLabel,
  statusTone,
} from "@/domain/rag/components/RagShared";

export function SourceDataTable({ pagination, onSelectSource }: { pagination: ReturnType<typeof useTablePagination<SourceRecord>>; onSelectSource: (source: SourceRecord) => void }) {
  const { t } = useTranslation("pages");
  const { formatNumber } = useLocaleFormatters();
  const tableT = (key: string, defaultValue: string) => t(`rag.dataTables.sources.${key}`, { defaultValue });

  return (
    <DataTable
      title={tableT("title", "Sources")}
      rows={pagination.visibleItems}
      rowKey={(source) => source.code}
      onRowClick={onSelectSource}
      pagination={<TablePagination itemLabel={tableT("itemLabel", "sources")} {...pagination.paginationProps} />}
      columns={[
        {
          id: "source",
          header: tableT("columns.source", "Source"),
          render: (source) => (
            <div>
              <p className="font-medium">{source.sourceType}</p>
              <p className="break-all text-[13px] text-[var(--color-on-surface-variant)]">{source.code}</p>
            </div>
          ),
        },
        { id: "corpus", header: tableT("columns.corpus", "Corpus"), render: (source) => source.corpus },
        { id: "owner", header: tableT("columns.owner", "Owner"), render: (source) => source.owner },
        { id: "units", header: tableT("columns.units", "Units"), align: "right", render: (source) => formatNumber(source.textUnits) },
        { id: "indexed", header: tableT("columns.indexed", "Indexed"), render: (source) => source.indexedAt },
        { id: "status", header: tableT("columns.status", "Status"), render: (source) => <Badge tone={statusTone(source.status)}>{formatRagStatusLabel(source.status, t)}</Badge> },
      ]}
    />
  );
}

export function TextUnitDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<TextUnitRecord>> }) {
  const { t } = useTranslation("pages");
  const { formatNumber } = useLocaleFormatters();
  const tableT = (key: string, defaultValue: string) => t(`rag.dataTables.textUnits.${key}`, { defaultValue });

  return (
    <DataTable
      title={tableT("title", "Text units")}
      rows={pagination.visibleItems}
      rowKey={(unit) => unit.id}
      pagination={<TablePagination itemLabel={tableT("itemLabel", "text units")} {...pagination.paginationProps} />}
      columns={[
        { id: "id", header: tableT("columns.id", "ID"), render: (unit) => <span className="font-mono text-[13px]">{unit.id}</span> },
        { id: "source", header: tableT("columns.source", "Source"), render: (unit) => <span className="break-all">{unit.source}</span> },
        { id: "preview", header: tableT("columns.preview", "Preview"), render: (unit) => <span className="text-[var(--color-on-surface-variant)]">{unit.preview}</span> },
        { id: "tokens", header: tableT("columns.tokens", "Tokens"), align: "right", render: (unit) => formatNumber(unit.tokens) },
        { id: "status", header: tableT("columns.vectorStatus", "Vector status"), render: (unit) => <Badge tone={statusTone(unit.embeddingStatus)}>{formatRagStatusLabel(unit.embeddingStatus, t)}</Badge> },
      ]}
    />
  );
}

export function WarningDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<WarningRecord>> }) {
  const { t } = useTranslation("pages");
  const tableT = (key: string, defaultValue: string) => t(`rag.dataTables.warnings.${key}`, { defaultValue });

  return (
    <DataTable
      title={tableT("title", "Warnings")}
      rows={pagination.visibleItems}
      rowKey={(warning) => `${warning.source}-${warning.stage}-${warning.age}`}
      pagination={<TablePagination itemLabel={tableT("itemLabel", "warnings")} {...pagination.paginationProps} />}
      columns={[
        { id: "severity", header: tableT("columns.severity", "Severity"), render: (warning) => <Badge tone={statusTone(warning.severity)}>{formatRagStatusLabel(warning.severity, t)}</Badge> },
        { id: "source", header: tableT("columns.source", "Source"), render: (warning) => <span className="break-all">{warning.source}</span> },
        { id: "stage", header: tableT("columns.stage", "Stage"), render: (warning) => warning.stage },
        { id: "message", header: tableT("columns.message", "Message"), render: (warning) => <span className="text-[var(--color-on-surface-variant)]">{warning.message}</span> },
        { id: "age", header: tableT("columns.age", "Age"), align: "right", render: (warning) => warning.age },
      ]}
    />
  );
}

export function TraceDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<TraceRecord>> }) {
  const { t } = useTranslation("pages");
  const { formatNumber } = useLocaleFormatters();
  const tableT = (key: string, defaultValue: string) => t(`rag.dataTables.traces.${key}`, { defaultValue });

  return (
    <DataTable
      title={tableT("title", "Retrieval traces")}
      rows={pagination.visibleItems}
      rowKey={(trace) => trace.id}
      pagination={<TablePagination itemLabel={tableT("itemLabel", "traces")} {...pagination.paginationProps} />}
      columns={[
        { id: "id", header: tableT("columns.id", "Trace"), render: (trace) => <span className="font-mono text-[13px]">{trace.id}</span> },
        { id: "query", header: tableT("columns.query", "Query"), render: (trace) => trace.query },
        { id: "profile", header: tableT("columns.profile", "Profile"), render: (trace) => trace.profile },
        { id: "route", header: tableT("columns.route", "Route"), render: (trace) => trace.route },
        { id: "hits", header: tableT("columns.hits", "Hits"), align: "right", render: (trace) => formatNumber(trace.hits) },
        { id: "tokens", header: tableT("columns.tokens", "Tokens"), align: "right", render: (trace) => formatNumber(trace.tokens) },
      ]}
    />
  );
}
