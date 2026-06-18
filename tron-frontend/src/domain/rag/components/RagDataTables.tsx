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
import {
  formatRagStatusLabel,
  statusTone,
} from "@/domain/rag/components/RagShared";

export function SourceDataTable({ pagination, onSelectSource }: { pagination: ReturnType<typeof useTablePagination<SourceRecord>>; onSelectSource: (source: SourceRecord) => void }) {
  return (
    <DataTable
      title="来源"
      rows={pagination.visibleItems}
      rowKey={(source) => source.code}
      onRowClick={onSelectSource}
      pagination={<TablePagination itemLabel="来源" {...pagination.paginationProps} />}
      columns={[
        {
          id: "source",
          header: "来源",
          render: (source) => (
            <div>
              <p className="font-medium">{source.sourceType}</p>
              <p className="break-all text-[13px] text-[var(--color-on-surface-variant)]">{source.code}</p>
            </div>
          ),
        },
        { id: "corpus", header: "语料库", render: (source) => source.corpus },
        { id: "owner", header: "归属", render: (source) => source.owner },
        { id: "units", header: "单元数", align: "right", render: (source) => source.textUnits.toLocaleString("zh-CN") },
        { id: "indexed", header: "索引时间", render: (source) => source.indexedAt },
        { id: "status", header: "状态", render: (source) => <Badge tone={statusTone(source.status)}>{formatRagStatusLabel(source.status)}</Badge> },
      ]}
    />
  );
}

export function TextUnitDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<TextUnitRecord>> }) {
  return (
    <DataTable
      title="文本单元"
      rows={pagination.visibleItems}
      rowKey={(unit) => unit.id}
      pagination={<TablePagination itemLabel="文本单元" {...pagination.paginationProps} />}
      columns={[
        { id: "id", header: "标识", render: (unit) => <span className="font-mono text-[13px]">{unit.id}</span> },
        { id: "source", header: "来源", render: (unit) => <span className="break-all">{unit.source}</span> },
        { id: "preview", header: "预览", render: (unit) => <span className="text-[var(--color-on-surface-variant)]">{unit.preview}</span> },
        { id: "tokens", header: "令牌数", align: "right", render: (unit) => unit.tokens.toLocaleString("zh-CN") },
        { id: "status", header: "向量状态", render: (unit) => <Badge tone={statusTone(unit.embeddingStatus)}>{formatRagStatusLabel(unit.embeddingStatus)}</Badge> },
      ]}
    />
  );
}

export function WarningDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<WarningRecord>> }) {
  return (
    <DataTable
      title="告警"
      rows={pagination.visibleItems}
      rowKey={(warning) => `${warning.source}-${warning.stage}-${warning.age}`}
      pagination={<TablePagination itemLabel="告警" {...pagination.paginationProps} />}
      columns={[
        { id: "severity", header: "级别", render: (warning) => <Badge tone={statusTone(warning.severity)}>{formatRagStatusLabel(warning.severity)}</Badge> },
        { id: "source", header: "来源", render: (warning) => <span className="break-all">{warning.source}</span> },
        { id: "stage", header: "阶段", render: (warning) => warning.stage },
        { id: "message", header: "消息", render: (warning) => <span className="text-[var(--color-on-surface-variant)]">{warning.message}</span> },
        { id: "age", header: "时间", align: "right", render: (warning) => warning.age },
      ]}
    />
  );
}

export function TraceDataTable({ pagination }: { pagination: ReturnType<typeof useTablePagination<TraceRecord>> }) {
  return (
    <DataTable
      title="检索追踪"
      rows={pagination.visibleItems}
      rowKey={(trace) => trace.id}
      pagination={<TablePagination itemLabel="追踪记录" {...pagination.paginationProps} />}
      columns={[
        { id: "id", header: "追踪", render: (trace) => <span className="font-mono text-[13px]">{trace.id}</span> },
        { id: "query", header: "查询", render: (trace) => trace.query },
        { id: "profile", header: "配置", render: (trace) => trace.profile },
        { id: "route", header: "路由", render: (trace) => trace.route },
        { id: "hits", header: "命中数", align: "right", render: (trace) => trace.hits.toLocaleString("zh-CN") },
        { id: "tokens", header: "令牌数", align: "right", render: (trace) => trace.tokens.toLocaleString("zh-CN") },
      ]}
    />
  );
}
