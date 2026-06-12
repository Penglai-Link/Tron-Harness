import {
  BookCopy,
  FileArchive,
  Filter,
  Gauge,
  Network,
  Search,
} from "lucide-react";

import {
  MetricCard,
  SegmentedTabs,
  SurfaceCard,
  TextInput,
  useTablePagination,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import {
  sources,
  textUnits,
} from "@/domain/rag/mock/data";
import {
  dataViews,
  type DataView,
  type SourceRecord,
  type TextUnitRecord,
  type TraceRecord,
  type WarningRecord,
} from "@/domain/rag/model/RagModel";
import {
  SourceDataTable,
  TextUnitDataTable,
  TraceDataTable,
  WarningDataTable,
} from "@/domain/rag/components/RagDataTables";

export function RagDataTab({
  dataView,
  query,
  sourcePagination,
  textUnitPagination,
  tracePagination,
  warningPagination,
  onDataViewChange,
  onQueryChange,
  onSelectSource,
}: {
  dataView: DataView;
  query: string;
  sourcePagination: ReturnType<typeof useTablePagination<SourceRecord>>;
  textUnitPagination: ReturnType<typeof useTablePagination<TextUnitRecord>>;
  tracePagination: ReturnType<typeof useTablePagination<TraceRecord>>;
  warningPagination: ReturnType<typeof useTablePagination<WarningRecord>>;
  onDataViewChange: (view: DataView) => void;
  onQueryChange: (value: string) => void;
  onSelectSource: (source: SourceRecord) => void;
}) {
  return (
    <div>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard icon={<FileArchive size={18} aria-hidden="true" />} label="Sources" value={String(sources.length)} />
        <MetricCard icon={<BookCopy size={18} aria-hidden="true" />} label="Text Units" value={String(textUnits.length)} />
        <MetricCard icon={<Gauge size={18} aria-hidden="true" />} label="Vector Coverage" value="94%" />
        <MetricCard icon={<Network size={18} aria-hidden="true" />} label="Search Traces 7d" value="318" />
      </div>

      <SurfaceCard className="mt-5">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <SegmentedTabs items={dataViews} value={dataView} onValueChange={(value) => onDataViewChange(value as DataView)} />
          <div className="flex min-w-0 flex-1 flex-col gap-3 sm:flex-row sm:items-center xl:max-w-xl">
            <div className="relative min-w-[260px] flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
              <TextInput
                className="w-full pl-9"
                placeholder="Search sources, chunks, warnings, or traces..."
                type="search"
                value={query}
                onChange={(event) => onQueryChange(event.target.value)}
              />
            </div>
            <Button size="icon" variant="secondary" aria-label="Open RAG data filters">
              <Filter size={17} aria-hidden="true" />
            </Button>
          </div>
        </div>
      </SurfaceCard>

      <div className="mt-5">
        {dataView === "sources" ? <SourceDataTable pagination={sourcePagination} onSelectSource={onSelectSource} /> : null}
        {dataView === "textUnits" ? <TextUnitDataTable pagination={textUnitPagination} /> : null}
        {dataView === "warnings" ? <WarningDataTable pagination={warningPagination} /> : null}
        {dataView === "traces" ? <TraceDataTable pagination={tracePagination} /> : null}
      </div>
    </div>
  );
}
