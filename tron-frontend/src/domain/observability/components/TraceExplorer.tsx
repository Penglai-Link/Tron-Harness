import { useEffect, useState } from "react";
import { useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { RefreshCcw, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import { useToast } from "@/components/ui/toast";
import { PageHeader, useLocaleFormatters } from "@/shared";
import {
  useTraceList,
  useTraceDetail,
  useTraceMetrics,
} from "../hooks/useObservability";
import { TraceStatus } from "./TraceStatus";
import { TraceInspector } from "./TraceInspector";
import { RuntimePanel } from "./RuntimePanel";
import { getCollectorExportConfig } from "../api/observabilityApi";
import type { TraceSummary } from "../model/Trace";

export function TraceExplorer({ workspaceCode }: { workspaceCode: string }) {
  const { t } = useTranslation("observability");
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const [params, setParams] = useSearchParams();
  const sessionCode = params.get("sessionCode") ?? "";
  const traceId = params.get("traceId") ?? "";
  const selectedId = params.get("selected") ?? "";
  const [sessionInput, setSessionInput] = useState(sessionCode);
  const [traceInput, setTraceInput] = useState(traceId);
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const toast = useToast();
  const list = useTraceList({
    workspaceCode,
    sessionCode,
    traceId,
    current,
    size,
  });
  const detail = useTraceDetail(workspaceCode, selectedId);
  const metrics = useTraceMetrics(workspaceCode, sessionCode, traceId);
  useEffect(() => {
    setSessionInput(sessionCode);
    setTraceInput(traceId);
    setCurrent(1);
  }, [sessionCode, traceId]);
  useEffect(() => {
    if (list.error) toast.error(list.error.message);
  }, [list.error, toast]);
  useEffect(() => {
    if (detail.error) toast.error(detail.error.message);
  }, [detail.error, toast]);
  const page = list.data;
  const total = page?.total ?? 0;
  const select = (id: string) => {
    const next = new URLSearchParams(params);
    next.set("selected", id);
    setParams(next, { replace: true });
  };
  const filterSession = (code: string) => {
    const next = new URLSearchParams();
    next.set("sessionCode", code);
    setParams(next);
  };
  const search = (event: React.FormEvent) => {
    event.preventDefault();
    const id = traceInput.trim().toLowerCase();
    if (id && (!/^[a-f0-9]{32}$/.test(id) || /^0+$/.test(id))) {
      toast.error(t("invalidTraceId"));
      return;
    }
    const next = new URLSearchParams();
    if (sessionInput.trim()) next.set("sessionCode", sessionInput.trim());
    if (id) next.set("traceId", id);
    setParams(next);
    setCurrent(1);
  };
  const copyCollectorConfig = async () => {
    try {
      const config = await getCollectorExportConfig();
      await navigator.clipboard.writeText(
        JSON.stringify(
          {
            OTEL_EXPORTER_OTLP_ENDPOINT: config.endpoint,
            OTEL_EXPORTER_OTLP_PROTOCOL: "http/protobuf",
            OTEL_EXPORTER_OTLP_HEADERS: `authorization=${encodeURIComponent(
              config.authorization
            )}`,
          },
          null,
          2
        )
      );
      toast.success(t("copied"));
    } catch {
      toast.error(t("copyFailed"));
    }
  };
  const refresh = () => {
    void metrics.refetch();
    void list.refetch();
    if (selectedId) void detail.refetch();
  };
  return (
    <>
      <PageHeader
        title={t("title")}
        actions={
          <>
            <InfoTooltip ariaLabel={t("help")}>{t("pageHelp")}</InfoTooltip>
            <Button
              variant="secondary"
              onClick={() => void copyCollectorConfig()}
            >
              {t("copyCollectorConfig")}
            </Button>
            <Button
              variant="secondary"
              onClick={refresh}
              disabled={!workspaceCode || list.isFetching}
            >
              <RefreshCcw
                className={list.isFetching ? "animate-spin" : undefined}
              />
              {t("refresh")}
            </Button>
          </>
        }
      />
      <RuntimePanel />
      <section
        className="mb-6 flex flex-wrap items-center gap-x-10 gap-y-5 border-b border-[var(--color-outline-variant)] pb-6"
        aria-label={t("overview")}
      >
        <dl className="flex flex-1 flex-wrap gap-x-10 gap-y-5">
          {[
            ["traceCount", metrics.data?.traceCount],
            ["failedTraces", metrics.data?.failedTraceCount],
            [
              "totalTokens",
              metrics.data
                ? metrics.data.inputTokens + metrics.data.outputTokens
                : undefined,
            ],
            ["averageDuration", metrics.data?.averageDurationMs],
            ["p95Duration", metrics.data?.p95DurationMs],
          ].map(([key, value]) => (
            <div key={key}>
              <dt className="text-xs text-[var(--color-on-surface-variant)]">
                {t(String(key))}
              </dt>
              <dd className="mt-2 text-2xl font-semibold tabular-nums">
                {typeof value === "number"
                  ? `${formatNumber(value, { maximumFractionDigits: 1 })}${
                      String(key).endsWith("Duration") ? " ms" : ""
                    }`
                  : "—"}
              </dd>
            </div>
          ))}
        </dl>
        <InfoTooltip ariaLabel={t("overviewHelpLabel")}>
          {t("overviewHelp")}
        </InfoTooltip>
        {metrics.isError && (
          <Button variant="secondary" onClick={() => void metrics.refetch()}>
            {t("retry")}
          </Button>
        )}
      </section>
      <DataTable<TraceSummary>
        title={t("traces")}
        rows={page?.records ?? []}
        rowKey={(row) => row.traceId}
        isRowSelected={(row) => row.traceId === selectedId}
        toolbar={
          <form
            className="flex min-w-0 flex-wrap items-center gap-3"
            onSubmit={search}
          >
            <Input
              aria-label={t("sessionCode")}
              placeholder={t("sessionCode")}
              className="w-48"
              value={sessionInput}
              onChange={(e) => setSessionInput(e.target.value)}
              maxLength={256}
            />
            <Input
              aria-label="Trace ID"
              placeholder="Trace ID"
              className="w-72 max-w-full font-mono"
              value={traceInput}
              onChange={(e) => setTraceInput(e.target.value)}
              maxLength={128}
            />
            <Button type="submit" disabled={!workspaceCode}>
              <Search />
              {t("search")}
            </Button>
            {(sessionCode || traceId) && (
              <Button
                variant="ghost"
                type="button"
                onClick={() => {
                  setParams({});
                  setSessionInput("");
                  setTraceInput("");
                }}
              >
                {t("reset")}
              </Button>
            )}
          </form>
        }
        emptyState={
          <span
            role="status"
            className="block w-fit max-w-[calc(100vw-6rem)] text-left"
          >
            {!workspaceCode
              ? t("chooseWorkspace")
              : list.isPending
              ? t("loading")
              : list.isError
              ? t("loadFailed")
              : t("noTraces")}
          </span>
        }
        pagination={
          <TablePagination
            totalItems={total}
            startItem={total ? (current - 1) * size + 1 : 0}
            endItem={
              total ? (current - 1) * size + (page?.records.length ?? 0) : 0
            }
            page={current}
            pageCount={Math.max(1, Math.ceil(total / size))}
            pageSize={size}
            onPageChange={(value) =>
              setCurrent(
                Math.max(
                  1,
                  Math.min(value, Math.max(1, Math.ceil(total / size)))
                )
              )
            }
            onPageSizeChange={(value) => {
              setSize(value);
              setCurrent(1);
            }}
            itemLabel={t("traces")}
          />
        }
        columns={[
          {
            id: "trace",
            header: "Trace ID",
            render: (row) => (
              <Button
                className="h-auto p-0 font-mono text-xs"
                variant="link"
                onClick={() => select(row.traceId)}
              >
                {row.traceId}
              </Button>
            ),
          },
          {
            id: "session",
            header: t("sessionCode"),
            render: (row) => (
              <Button
                className="h-auto max-w-48 truncate p-0 text-xs"
                variant="link"
                onClick={() => filterSession(row.sessionCode)}
              >
                {row.sessionCode}
              </Button>
            ),
          },
          { id: "agent", header: t("agent"), render: (row) => row.agentCode },
          {
            id: "status",
            header: t("state"),
            render: (row) => <TraceStatus status={row.status} />,
          },
          {
            id: "start",
            header: t("startedAt"),
            render: (row) => (
              <span className="whitespace-nowrap tabular-nums">
                {formatDateTime(row.startedAt, {
                  dateStyle: "short",
                  timeStyle: "medium",
                })}
              </span>
            ),
          },
          {
            id: "duration",
            header: t("duration"),
            align: "right",
            render: (row) => (
              <span className="tabular-nums">
                {formatNumber(row.durationMs, { maximumFractionDigits: 2 })} ms
              </span>
            ),
          },
          {
            id: "spans",
            header: "Spans",
            align: "right",
            render: (row) => formatNumber(row.spanCount),
          },
          {
            id: "tokens",
            header: "Tokens",
            align: "right",
            render: (row) => formatNumber(row.inputTokens + row.outputTokens),
          },
          {
            id: "errors",
            header: t("errors"),
            align: "right",
            render: (row) => (
              <span
                className={
                  row.errorCount ? "text-[var(--color-error)]" : undefined
                }
              >
                {formatNumber(row.errorCount)}
              </span>
            ),
          },
        ]}
      />
      {detail.isPending && selectedId && (
        <div role="status" className="py-8 text-center text-sm">
          {t("loading")}
        </div>
      )}
      {detail.isError && selectedId && (
        <div
          className="flex items-center justify-center gap-3 py-8 text-sm"
          role="alert"
        >
          {t("loadFailed")}
          <Button variant="secondary" onClick={() => void detail.refetch()}>
            {t("retry")}
          </Button>
        </div>
      )}
      {detail.data && (
        <TraceInspector key={detail.data.trace.traceId} detail={detail.data} />
      )}
    </>
  );
}
