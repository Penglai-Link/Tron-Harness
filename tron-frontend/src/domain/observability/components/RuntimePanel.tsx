import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import { useLocaleFormatters } from "@/shared";
import { useRuntimeMetrics, useMetricPoints } from "../hooks/useObservability";
import type { MetricPoint } from "../model/Trace";
export function RuntimePanel() {
  const { t } = useTranslation("observability");
  const { formatNumber, formatDateTime } = useLocaleFormatters();
  const runtime = useRuntimeMetrics();
  const [show, setShow] = useState(false);
  return (
    <section
      className="mb-6 border-b border-[var(--color-outline-variant)] pb-6"
      aria-label={t("runtime")}
    >
      <div className="flex flex-wrap items-center justify-between gap-5">
        <dl className="flex flex-wrap gap-x-10 gap-y-5">
          <div>
            <dt className="text-xs text-[var(--color-on-surface-variant)]">
              {t("processCpu")}
            </dt>
            <dd className="mt-2 text-2xl font-semibold tabular-nums">
              {runtime.data?.cpuPercent != null
                ? `${formatNumber(runtime.data.cpuPercent, {
                    maximumFractionDigits: 1,
                  })}%`
                : "—"}
            </dd>
          </div>
          <div>
            <dt className="text-xs text-[var(--color-on-surface-variant)]">
              {t("processMemory")}
            </dt>
            <dd className="mt-2 text-2xl font-semibold tabular-nums">
              {runtime.data?.memoryBytes != null
                ? `${formatNumber(runtime.data.memoryBytes / 1048576, {
                    maximumFractionDigits: 1,
                  })} MiB`
                : "—"}
            </dd>
          </div>
          <div>
            <dt className="text-xs text-[var(--color-on-surface-variant)]">
              {t("metricPoints")}
            </dt>
            <dd className="mt-2 text-2xl font-semibold tabular-nums">
              {runtime.data ? formatNumber(runtime.data.metricPointCount) : "—"}
            </dd>
          </div>
        </dl>
        <div className="flex items-center gap-3">
          <InfoTooltip ariaLabel={t("runtimeHelpLabel")}>
            {t("runtimeHelp")}
          </InfoTooltip>
          <Button
            variant="secondary"
            onClick={() => setShow(!show)}
            aria-expanded={show}
          >
            {t(show ? "hideMetrics" : "showMetrics")}
          </Button>
          {runtime.isError && (
            <Button variant="secondary" onClick={() => void runtime.refetch()}>
              {t("retry")}
            </Button>
          )}
        </div>
      </div>
      {runtime.data?.sampledAt && (
        <time className="mt-3 block text-xs text-[var(--color-on-surface-variant)]">
          {formatDateTime(runtime.data.sampledAt, { timeStyle: "medium" })}
        </time>
      )}
      {show && <MetricPointsTable />}
    </section>
  );
}
function MetricPointsTable() {
  const { t } = useTranslation("observability");
  const { formatNumber, formatDateTime } = useLocaleFormatters();
  const [input, setInput] = useState("");
  const [name, setName] = useState("");
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(15);
  const [selected, setSelected] = useState<MetricPoint | null>(null);
  const query = useMetricPoints(name, page, size);
  const total = query.data?.total ?? 0;
  return (
    <div className="mt-5 min-w-0">
      <DataTable<MetricPoint>
        title={t("metricPoints")}
        rows={query.data?.records ?? []}
        rowKey={(p) => p.pointId}
        toolbar={
          <form
            className="flex flex-wrap gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              setName(input.trim());
              setPage(1);
              setSelected(null);
            }}
          >
            <Input
              className="w-72 max-w-full"
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
            {query.isPending
              ? t("loading")
              : query.isError
              ? t("loadFailed")
              : t("empty")}
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
          { id: "kind", header: t("metricKind"), render: (p) => p.kind },
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
          { id: "unit", header: t("metricUnit"), render: (p) => p.unit || "—" },
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
            {selected.description && (
              <InfoTooltip ariaLabel={t("help")}>
                {selected.description}
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
