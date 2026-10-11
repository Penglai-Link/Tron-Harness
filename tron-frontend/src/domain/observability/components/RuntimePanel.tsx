import { PlatformWorkspaceFilter } from "@/domain/settings/components/PlatformWorkspaceScope";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { TextInput } from "@/components/forms";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import { useLocaleFormatters } from "@/shared";
import { useMetricPoints } from "../hooks/useObservability";
import type { MetricPoint } from "../model/Trace";
export function RuntimePanel() {
  const { t } = useTranslation("observability");
  const { formatNumber, formatDateTime } = useLocaleFormatters();
  const [input, setInput] = useState("");
  const [name, setName] = useState("");
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(15);
  const [selected, setSelected] = useState<MetricPoint | null>(null);
  const query = useMetricPoints(name, page, size);
  const total = query.data?.total ?? 0;
  const description = (point: MetricPoint) =>
    point.name === "process.cpu.utilization"
      ? t("cpuDescription")
      : point.name === "process.memory.usage"
      ? t("memoryDescription")
      : point.description || t("noMetricDescription");
  const unit = (point: MetricPoint) =>
    point.unit === "1"
      ? t(
          point.name === "process.cpu.utilization"
            ? "cpuRatioUnit"
            : "dimensionlessUnit"
        )
      : point.unit === "By"
      ? t("bytesUnit")
      : point.unit || t("unspecifiedUnit");
  return (
    <div className="h-full min-h-0 min-w-0">
      <DataTable<MetricPoint>
        className="flex max-h-full flex-col [&>div]:shrink-0 [&>[data-slot=table-container]]:min-h-0 [&>[data-slot=table-container]]:shrink [&>[data-slot=table-container]]:overflow-auto [&_thead]:sticky [&_thead]:top-0 [&_thead]:z-10 [&_thead]:bg-[var(--color-elevated-surface)]"
        title={t("metricPoints")}
        rows={query.data?.records ?? []}
        rowKey={(p) => p.pointId}
        toolbar={
          <form
            className="flex flex-wrap items-center gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              if (page === 1 && name === input.trim()) void query.refetch();
              setName(input.trim());
              setPage(1);
              setSelected(null);
            }}
          >
            <PlatformWorkspaceFilter />
            <TextInput
              className="h-[var(--tron-input-height,2.25rem)] w-48 max-w-full sm:h-[var(--tron-input-height,2.25rem)]"
              aria-label={t("metricName")}
              placeholder={t("metricName")}
              value={input}
              onChange={(e) => setInput(e.target.value)}
            />
            <Button type="submit">{t("search")}</Button>
          </form>
        }
        emptyState={
          <span role="status">
            {query.isPending || query.isError ? null : t("empty")}
          </span>
        }
        pagination={
          <TablePagination
            totalItems={total}
            startItem={total ? (page - 1) * size + 1 : 0}
            endItem={total ? Math.min(page * size, total) : 0}
            page={page}
            pageCount={Math.max(1, Math.ceil(total / size))}
            pageSize={size}
            onPageChange={setPage}
            onPageSizeChange={(s) => {
              setSize(s);
              setPage(1);
            }}
            itemLabel={t("metricPoints")}
          />
        }
        columns={[
          {
            id: "name",
            header: t("metricName"),
            render: (p) => (
              <Button
                variant="link"
                className="h-auto p-0 font-mono text-xs"
                onClick={() => setSelected(p)}
              >
                {p.name}
              </Button>
            ),
          },
          {
            id: "description",
            header: t("metricDescription"),
            cellClassName:
              "whitespace-normal min-w-56 max-w-80 text-xs text-[var(--color-on-surface-variant)]",
            render: description,
          },
          {
            id: "kind",
            header: (
              <span className="inline-flex items-center gap-1.5">
                {t("metricKind")}
                <InfoTooltip ariaLabel={t("metricKindHelpLabel")}>
                  <div className="max-w-80 space-y-2 whitespace-normal">
                    {[
                      "gaugeHelp",
                      "sumHelp",
                      "histogramHelp",
                      "exponentialHistogramHelp",
                      "summaryHelp",
                    ].map((key) => (
                      <p key={key}>{t(key)}</p>
                    ))}
                  </div>
                </InfoTooltip>
              </span>
            ),
            render: (p) => p.kind,
          },
          {
            id: "value",
            header: t("metricValue"),
            render: (p) =>
              typeof p.data.asDouble === "number"
                ? formatNumber(p.data.asDouble, { maximumFractionDigits: 4 })
                : p.data.asInt != null
                ? String(p.data.asInt)
                : p.data.count != null
                ? `${t("metricCount")}: ${String(p.data.count)}`
                : "—",
          },
          {
            id: "unit",
            header: (
              <span className="inline-flex items-center gap-1.5">
                {t("metricUnit")}
                <InfoTooltip ariaLabel={t("metricUnitHelpLabel")}>
                  <div className="max-w-80 space-y-2 whitespace-normal">
                    {[
                      "dimensionlessHelp",
                      "cpuRatioHelp",
                      "bytesHelp",
                      "timeUnitsHelp",
                      "otherUnitsHelp",
                    ].map((key) => (
                      <p key={key}>{t(key)}</p>
                    ))}
                  </div>
                </InfoTooltip>
              </span>
            ),
            render: unit,
          },
          {
            id: "time",
            header: t("sampledAt"),
            render: (p) => (
              <span className="whitespace-nowrap">
                {formatDateTime(p.timestamp, {
                  dateStyle: "short",
                  timeStyle: "medium",
                })}
              </span>
            ),
          },
        ]}
      />
      {selected && (
        <div className="mt-4 min-w-0 rounded-[var(--tron-card-radius,8px)] border border-[var(--color-outline-variant)] p-4">
          <div className="mb-3 flex items-center gap-3">
            <h3 className="break-all font-mono text-sm">{selected.name}</h3>
            {description(selected) && (
              <InfoTooltip ariaLabel={t("help")}>
                {description(selected)}
              </InfoTooltip>
            )}
            <InfoTooltip ariaLabel={t("help")}>
              {t("metricDataHelp")}
            </InfoTooltip>
          </div>
          <pre className="max-h-96 overflow-auto text-xs">
            {JSON.stringify(
              {
                resource: selected.resource,
                scope: JSON.parse(selected.scope),
                kind: selected.kind,
                temporality: selected.temporality,
                monotonic: selected.monotonic,
                data: selected.data,
              },
              null,
              2
            )}
          </pre>
        </div>
      )}
    </div>
  );
}
