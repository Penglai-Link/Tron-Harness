import {
  BookCopy,
  FileArchive,
  Filter,
  Gauge,
  Network,
  Search,
} from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import {
  MetricCard,
  SegmentedTabs,
  SurfaceCard,
  TextInput,
  useLocaleFormatters,
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
  const { t } = useTranslation("pages");
  const { formatNumber } = useLocaleFormatters();
  const pageT = (key: string, defaultValue: string) => t(`rag.data.${key}`, { defaultValue });
  const translatedDataViews = useMemo(() => dataViews.map((view) => ({
    ...view,
    label: pageT(`views.${view.value}`, view.label),
  })), [t]);

  return (
    <div>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard icon={<FileArchive size={18} aria-hidden="true" />} label={pageT("metrics.sources", "Sources")} value={formatNumber(sources.length)} />
        <MetricCard icon={<BookCopy size={18} aria-hidden="true" />} label={pageT("metrics.textUnits", "Text units")} value={formatNumber(textUnits.length)} />
        <MetricCard icon={<Gauge size={18} aria-hidden="true" />} label={pageT("metrics.vectorCoverage", "Vector coverage")} value="94%" />
        <MetricCard icon={<Network size={18} aria-hidden="true" />} label={pageT("metrics.weeklyTraces", "Last 7 days retrieval traces")} value={formatNumber(318)} />
      </div>

      <SurfaceCard className="mt-5">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <SegmentedTabs items={translatedDataViews} value={dataView} onValueChange={(value) => onDataViewChange(value as DataView)} />
          <div className="flex min-w-0 flex-1 flex-col gap-3 sm:flex-row sm:items-center xl:max-w-xl">
            <div className="relative min-w-[260px] flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
              <TextInput
                className="w-full pl-9"
                placeholder={pageT("searchPlaceholder", "Search sources, chunks, warnings, or traces...")}
                type="search"
                value={query}
                onChange={(event) => onQueryChange(event.target.value)}
              />
            </div>
            <Button size="icon" variant="secondary" aria-label={pageT("filterAria", "Open RAG data filters")}>
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
