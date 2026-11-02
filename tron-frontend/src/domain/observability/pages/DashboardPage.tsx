import { PlatformWorkspaceFilter, usePlatformDataHref } from "@/domain/settings/components/PlatformWorkspaceScope";
import { PluginCountCard, ResourceTrendCards } from "../components/ResourceTrendCards";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import {
  SurfaceCard,
  useLocaleFormatters,
  useWorkspaceSpaces,
} from "@/shared";
import { useObservabilityDashboard } from "../hooks/useObservability";
import { CollectorConfigButton } from "../components/CollectorConfigButton";
import { ServiceTopology } from "../components/ServiceTopology";
import {
  formatTraceDuration,
  formatTraceTokens,
} from "../components/traceFormat";

export function DashboardPage() {
  const { t } = useTranslation("observability");
  const { formatNumber, formatDateTime, currentLocale } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspace =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const href = usePlatformDataHref();
  const query = useObservabilityDashboard(workspace);
  const data = query.data;
  const n = (value: number | undefined) =>
    value == null ? "—" : formatNumber(value);
  const duration = (value: number | undefined) =>
    value == null ? "—" : formatTraceDuration(value, currentLocale);
  const errorRate = (failed: number, total: number) => total > 0
    ? `${formatNumber((failed / total) * 100, { maximumFractionDigits: 1 })}%`
    : "—";
  const cards = [
    ["requestCount", n(data?.traceCount), "requestCountHint"],
    [
      "errorRate",
      data ? errorRate(data.failedTraceCount, data.traceCount) : "—",
      "errorRateHint",
    ],
    ["averageDuration", duration(data?.averageDurationMs), "latencyHint"],
    ["p95Duration", duration(data?.p95DurationMs), "p95Hint"],
    ["runningRequests", n(data?.runningTraceCount), "runningHint"],
    ["cumulativeFailedTraces", n(data?.failedTraceCount), "failedHint"],
    [
      "totalTokens",
      data
        ? formatTraceTokens(data.inputTokens + data.outputTokens, currentLocale)
        : "—",
      "tokenHint",
    ],
    ["metricPoints", n(data?.metricPointCount), "metricPointHint"],
  ];
  const buckets = data?.recentRequests ?? [];
  const recent = buckets.reduce((sum, bucket) => ({
    requests: sum.requests + bucket.requests,
    errors: sum.errors + bucket.errors,
  }), { requests: 0, errors: 0 });
  const maximum = Math.max(1, ...buckets.map((b) => b.requests));
  return (
    <>
      <div className="flex flex-col gap-6 pb-8">
        <header className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
          <PlatformWorkspaceFilter />
          <div className="ml-auto flex flex-wrap items-center gap-3">
            {data && (
              <time className="inline-flex h-8 items-center whitespace-nowrap leading-none text-xs text-[var(--color-on-surface-variant)]">
                {t("updatedAt")}{" "}
                {formatDateTime(data.sampledAt, { timeStyle: "medium" })}
              </time>
            )}
            <CollectorConfigButton />
            <Button
              variant="secondary"
              disabled={query.isFetching}
              onClick={() => void query.refetch()}
            >
              {t("refresh")}
            </Button>
          </div>
        </header>
        <dl className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {cards.map(([label, value]) => (
            <div key={label}>
              <SurfaceCard className="!px-5 !py-4 !shadow-none">
              <dt className="flex items-center gap-1 text-xs text-[var(--color-on-surface-variant)]">
                {t(label)}
                {label === "errorRate" && <InfoTooltip variant="icon" ariaLabel={t("errorRate")}>{t("errorRateHint")}</InfoTooltip>}
              </dt>
              <dd className="mt-2 truncate text-2xl font-semibold leading-none tracking-tight tabular-nums" title={value}>
                {value}
              </dd>
              {label === "errorRate" && data && <p className="mt-2 text-xs tabular-nums text-[var(--color-on-surface-variant)]">
                {t("recentErrorRate", { rate: errorRate(recent.errors, recent.requests) })}
              </p>}
              </SurfaceCard>
            </div>
          ))}
        </dl>
          <SurfaceCard>
            <div className="mb-5 flex items-center justify-between gap-3">
              <div className="flex items-center gap-1">
                <h2 className="font-semibold">{t("requestTrend")}</h2>
                <InfoTooltip variant="icon" ariaLabel={t("requestTrend")}>{t("trendHint")}</InfoTooltip>
              </div>
              <Link className="text-xs underline" to={href("/settings/observability/traces")}>
                {t("viewTraces")}
              </Link>
            </div>
            {data ? (
              <>
                <div
                  className="flex h-36 items-end gap-2 border-b border-[var(--color-outline-variant)]"
                  role="img"
                  aria-label={t("requestTrend")}
                >
                  {buckets.map((b) => (
                    <div
                      key={b.startedAt}
                      className="flex h-full min-w-0 flex-1 items-end"
                      title={`${formatDateTime(b.startedAt, {
                        timeStyle: "short",
                      })} · ${t("requestsValue", { count: b.requests })} · ${t(
                        "errorsValue",
                        { count: b.errors }
                      )}`}
                    >
                      <div
                        className="relative w-full rounded-t bg-[var(--chart-1)]"
                        style={{ height: `${(b.requests / maximum) * 100}%` }}
                      >
                        <div
                          className="absolute inset-x-0 bottom-0 bg-[var(--chart-5)]"
                          style={{
                            height: `${
                              b.requests ? (b.errors / b.requests) * 100 : 0
                            }%`,
                          }}
                        />
                      </div>
                    </div>
                  ))}
                </div>
                <div className="mt-2 flex justify-between text-xs text-[var(--color-on-surface-variant)]">
                  <span>
                    {buckets[0] &&
                      formatDateTime(buckets[0].startedAt, {
                        timeStyle: "short",
                      })}
                  </span>
                  <span>
                    {t("requestsValue", {
                      count: buckets.reduce((sum, b) => sum + b.requests, 0),
                    })}
                  </span>
                  <span>
                    {buckets.at(-1) &&
                      formatDateTime(buckets.at(-1)!.startedAt, {
                        timeStyle: "short",
                      })}
                  </span>
                </div>
                <details className="mt-4 text-xs">
                  <summary className="cursor-pointer">{t("trendData")}</summary>
                  <ul className="mt-2 grid grid-cols-2 gap-2">
                    {buckets.map((b) => (
                      <li key={b.startedAt}>
                        {formatDateTime(b.startedAt, { timeStyle: "short" })} ·{" "}
                        {t("requestsValue", { count: b.requests })} ·{" "}
                        {t("errorsValue", { count: b.errors })}
                      </li>
                    ))}
                  </ul>
                </details>
              </>
            ) : (
              <div className="h-36" />
            )}
          </SurfaceCard>
        <div className="mt-4 flex flex-col gap-6 border-t-2 border-[var(--color-outline)] pt-10">
          <ResourceTrendCards />
          <div className="grid gap-6 lg:grid-cols-2">
          <PluginCountCard />
          <SurfaceCard className="flex flex-col">
            <div className="mb-4 flex items-center justify-between gap-3">
              <div className="flex items-center gap-1">
                <h2 className="font-semibold">{t("resourceUsage")}</h2>
                <InfoTooltip variant="icon" ariaLabel={t("resourceUsage")}>{t("runtimeHelp")}</InfoTooltip>
              </div>
              <Link className="text-xs text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] hover:underline" to={href("/settings/observability/metrics")}>
                {t("showMetrics")}
              </Link>
            </div>
            <dl className="divide-y divide-[var(--color-outline-variant)] rounded-lg bg-[var(--color-surface-container)] px-4">
              <div className="flex flex-wrap items-center justify-between gap-3 py-4">
                <dt className="text-sm text-[var(--color-on-surface-variant)]">
                  {t("processCpu")}
                </dt>
                <dd className="text-3xl font-semibold leading-none tracking-tight tabular-nums">
                  {data?.cpuPercent != null
                    ? <>{formatNumber(data.cpuPercent, { maximumFractionDigits: 1 })}<span className="ml-1 text-sm font-normal text-[var(--color-on-surface-variant)]">%</span></>
                    : "—"}
                </dd>
              </div>
              <div className="flex flex-wrap items-center justify-between gap-3 py-4">
                <dt className="text-sm text-[var(--color-on-surface-variant)]">
                  {t("processMemory")}
                </dt>
                <dd className="text-3xl font-semibold leading-none tracking-tight tabular-nums">
                  {data?.memoryBytes != null
                    ? <>{formatNumber(data.memoryBytes / 1048576, { maximumFractionDigits: 1 })}{" "}<span className="text-sm font-normal text-[var(--color-on-surface-variant)]">MiB</span></>
                    : "—"}
                </dd>
              </div>
            </dl>
            {data?.resourceSampledAt && (
              <p className="mt-4 flex justify-between gap-3 text-xs text-[var(--color-on-surface-variant)]">
                <span>{t("sampledAt")}</span>
                <time dateTime={new Date(data.resourceSampledAt).toISOString()} className="tabular-nums">
                  {formatDateTime(data.resourceSampledAt, { timeStyle: "medium" })}
                </time>
              </p>
            )}
          </SurfaceCard>
          </div>
        <SurfaceCard>
          <div className="mb-3 flex items-center justify-between gap-3">
            <div className="flex items-center gap-1">
                <h2 className="font-semibold">{t("serviceTopology")}</h2>
                <InfoTooltip variant="icon" ariaLabel={t("serviceTopology")}>{t("serviceTopologyHint")}</InfoTooltip>
              </div>
            <span className="text-xs text-[var(--color-on-surface-variant)]">
              {t("processCount", { count: data?.processes.length ?? 0 })}
            </span>
          </div>
          {data?.processes.length ? (
            <ServiceTopology
              processes={data.processes}
              sampledAt={data.sampledAt}
            />
          ) : (
            data ? <p className="text-sm">{t("noProcesses")}</p> : null
          )}
        </SurfaceCard>
        </div>
      </div>
    </>
  );
}
