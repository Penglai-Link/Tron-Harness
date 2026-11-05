import { TraceIdentifier } from "./TraceIdentifier";
import { triggerLabel } from "./traceTrigger";
import { Copy } from "lucide-react";
import { useMemo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";

import { TablePagination } from "@/components/table-pagination";
import { TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableRow,
  TableCell,
  TableHead,
  TableHeader,
} from "@/components/ui/table";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import { useToast } from "@/components/ui/toast";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import { contextModelDisplayName } from "@/domain/chat/model/contextAssemblyReport";
import type { ModelSelectOption } from "@/domain/model/model";
import type { TraceDetail, TraceSpan } from "../model/Trace";
import { TraceStatus } from "./TraceStatus";
import { formatTraceDuration, formatTraceTokens } from "./traceFormat";
import { SpanVisualization } from "./SpanVisualization";
import { traceTree } from "./traceTree";
import { TraceSpanLogs } from "./TraceSpanLogs";

export function TraceInspector({ detail, models = [] }: { detail: TraceDetail; models?: readonly ModelSelectOption[] }) {
  const { t, i18n } = useTranslation("observability");
  const { currentLocale, formatNumber, formatDateTime } = useLocaleFormatters();
  const toast = useToast();
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(25);
  const [keyword, setKeyword] = useState("");
  const rows = useMemo(() => traceTree(detail.spans), [detail.spans]);
  const modelLabel = (code: string) => contextModelDisplayName(code, models) || code;
  const operationLabel = (item: TraceSpan) => {
    const model = item.attributes["gen_ai.request.model"];
    return item.attributes["gen_ai.tool.name"] ?? (model ? modelLabel(model) : item.name);
  };
  const filteredRows = useMemo(() => {
    const query = keyword.trim().toLowerCase();
    return query ? rows.filter(({ span }) => `${span.name} ${span.spanId} ${span.attributes["gen_ai.tool.name"] ?? ""} ${span.attributes["gen_ai.request.model"] ?? ""} ${operationLabel(span)}`.toLowerCase().includes(query)) : rows;
  }, [rows, keyword, models]);
  const pageCount = Math.max(1, Math.ceil(filteredRows.length / pageSize));
  const currentPage = Math.min(page, pageCount);
  const visibleRows = useMemo(() => filteredRows.slice((currentPage - 1) * pageSize, currentPage * pageSize), [filteredRows, currentPage, pageSize]);
  const [selected, setSelected] = useState("");
  const [view, setView] = useState("table");
  const span =
    detail.spans.find((s) => s.spanId === selected) ??
    detail.spans.find((s) => s.name === "agent.run") ??
    detail.spans[0];
  const [detailTab, setDetailTab] = useState(() => span?.status === "Error" || span?.status === "Cancelled" ? "logs" : "attributes");
  const trace = detail.trace;
  const triggerRoot = detail.spans.find((item) => item.spanId === trace.rootSpanId)
    ?? detail.spans.find((item) => !item.parentSpanId);
  const isPageTrigger = triggerRoot?.attributes["tron.trace.page"] === "true" || Boolean(triggerRoot?.attributes["url.path"]);
  const triggerCategory = !isPageTrigger && (triggerRoot?.attributes["tron.agent.code"] || triggerRoot?.attributes["tron.dude.code"]) ? "Agent" : "Page";
  const title = triggerLabel(trace.trigger, i18n, triggerCategory);
  const isDudeExecution = detail.spans.some((item) => item.name === "tron.dude.execute");
  const canOpenSession = !isPageTrigger && Boolean(trace.sessionCode?.trim());
  const sessionHref = isDudeExecution
    ? `/dudes?${new URLSearchParams({ dude: trace.agentCode, tab: "history", session: trace.sessionCode })}`
    : `/chats?${new URLSearchParams({ sessionCode: trace.sessionCode, workspaceCode: trace.workspaceCode })}`;
  const duration = (value: number) => formatTraceDuration(value, currentLocale);
  const metrics = [
    ["duration", duration(trace.durationMs)],
    ["modelCalls", formatNumber(trace.modelCount)],
    ["toolCalls", formatNumber(trace.toolCount)],
    ["inputTokens", formatTraceTokens(trace.inputTokens, currentLocale)],
    ["cachedInputTokens", formatTraceTokens(trace.cachedInputTokens, currentLocale)],
    ["outputTokens", formatTraceTokens(trace.outputTokens, currentLocale)],
    ["traceTotalTokens", formatTraceTokens(trace.inputTokens + trace.outputTokens, currentLocale)],
    ["errors", formatNumber(trace.errorCount)],
  ];
  const pagination = (
    <TablePagination totalItems={filteredRows.length} startItem={filteredRows.length ? (currentPage - 1) * pageSize + 1 : 0} endItem={Math.min(currentPage * pageSize, filteredRows.length)} page={currentPage} pageCount={pageCount} pageSize={pageSize} pageSizeOptions={[15, 25, 50]} itemLabel="Spans" onPageChange={(value) => setPage(Math.max(1, Math.min(value, pageCount)))} onPageSizeChange={(value) => { setPageSize(value); setPage(1); }} />
  );
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(trace.traceId);
      toast.success(t("copied"));
    } catch {
      toast.error(t("copyFailed"));
    }
  };
  return (
    <section
      className="min-w-0 rounded-[var(--tron-card-radius,8px)] border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]"
      aria-label={t("detail")}
    >
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
        <div className="flex min-w-0 flex-wrap items-center gap-3">
          <h2 className="font-semibold">{title === "—" ? t("detail") : title}</h2>
          <code className="break-all text-xs">{trace.traceId}</code>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t("copyTraceId")}
            onClick={() => void copy()}
          >
            <Copy />
          </Button>
          <TraceStatus status={trace.status} />
        </div>
        <div className="flex items-center gap-2">
          {canOpenSession && <Button
            variant="secondary"
            nativeButton={false}
            render={<Link to={sessionHref} />}
          >

            {t("openSession")}
          </Button>}
          <InfoTooltip ariaLabel={t("help")}>{t("metricsHelp")}</InfoTooltip>
        </div>
      </header>
      <dl className="grid grid-cols-2 gap-5 border-b border-[var(--color-outline-variant)] p-5 sm:grid-cols-4 xl:grid-cols-8">
        {metrics.map(([key, value]) => (
          <div key={key}>
            <dt className="text-xs text-[var(--color-on-surface-variant)]">
              {t(key)}
            </dt>
            <dd className="mt-2 text-xl font-semibold tabular-nums">{value}</dd>
          </div>
        ))}
      </dl>
      <div className="grid min-w-0 xl:grid-cols-[minmax(0,1.5fr)_minmax(280px,1fr)]">
        <div className="min-w-0 p-5">
          <Tabs value={view} onValueChange={(value) => setView(String(value))}>
            <TabsList aria-label={t("detailView")}>
              <TabsTrigger value="table">{t("tableView")}</TabsTrigger>
              <TabsTrigger value="waterfall">{t("waterfall")}</TabsTrigger>
              <TabsTrigger value="topology">{t("topology")}</TabsTrigger>
            </TabsList>
            <div className="my-3 space-y-3">
              <h3 className="text-sm font-semibold">{t("spanList")}</h3>
              <TextInput aria-label={t("searchSpans")} placeholder={t("searchSpans")} value={keyword} onChange={(event) => { setKeyword(event.target.value); setPage(1); }} />
              {pagination}
            </div>
            <TabsContent value="table">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("operation")}</TableHead>
                    <TableHead>Span ID</TableHead>
                    <TableHead>{t("state")}</TableHead>
                    <TableHead>{t("elapsedTime")}</TableHead>
                    <TableHead>{t("duration")}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {visibleRows.map(({ span: item, depth }) => {
                    const operation = operationLabel(item);
                    return (
                    <TableRow
                      key={item.spanId}
                      data-state={
                        item.spanId === span?.spanId ? "selected" : undefined
                      }
                    >
                      <TableCell>
                        <Tooltip>
                          <TooltipTrigger render={<Button
                            variant="link"
                            className="h-auto max-w-64 justify-start p-0"
                            style={{ paddingLeft: Math.min(depth, 8) * 12 }}
                            onClick={() => setSelected(item.spanId)}
                          />}>
                            <span className="truncate">{operation}</span>
                          </TooltipTrigger>
                          <TooltipContent className="max-w-80 break-all">{operation}</TooltipContent>
                        </Tooltip>
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {item.spanId}
                      </TableCell>
                      <TableCell>
                        <TraceStatus status={item.status} />
                      </TableCell>
                      <TableCell>
                        {duration(item.startedAt - trace.startedAt)}
                      </TableCell>
                      <TableCell>{duration(item.durationMs)}</TableCell>
                    </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </TabsContent>
            <TabsContent value="waterfall">
              {visibleRows.length > 0 && (
                <SpanVisualization
                  detail={detail}
                  rows={visibleRows.map(({ span, depth }) => ({ span: { ...span, name: operationLabel(span) }, depth }))}
                  mode="waterfall"
                  selected={span?.spanId ?? ""}
                  onSelect={setSelected}
                />
              )}
            </TabsContent>
            <TabsContent value="topology">
              {visibleRows.length > 0 && (
                <SpanVisualization
                  detail={detail}
                  rows={visibleRows.map(({ span, depth }) => ({ span: { ...span, name: operationLabel(span) }, depth }))}
                  mode="topology"
                  selected={span?.spanId ?? ""}
                  onSelect={setSelected}
                />
              )}
            </TabsContent>
          </Tabs>
          <div className="mt-4 border-t border-[var(--color-outline-variant)] pt-4">
            {pagination}
          </div>
          {!filteredRows.length && (
            <p role="status" className="py-10 text-center text-sm">
              {t("noSpans")}
            </p>
          )}
        </div>
        {span && (
          <aside
            className="min-w-0 border-t border-[var(--color-outline-variant)] p-5 xl:border-l xl:border-t-0"
            aria-label={t("spanDetail")}
          >
            <div className="mb-3 flex min-w-0 flex-wrap items-start justify-between gap-2">
              <h3 className="min-w-0 flex-1 text-sm font-semibold [overflow-wrap:anywhere]">{operationLabel(span)}</h3>
              <TraceStatus status={span.status} />
            </div>
            <dl className="mb-5 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 text-xs">
              <dt>Span ID</dt>
              <dd><TraceIdentifier value={span.spanId} label="Span ID" className="w-fit max-w-full" /></dd>
              <dt>{t("parent")}</dt>
              <dd>{span.parentSpanId ? <TraceIdentifier value={span.parentSpanId} label={t("parent")} className="w-fit max-w-full" /> : "—"}</dd>
              <dt>{t("startedAt")}</dt>
              <dd>
                {formatDateTime(span.startedAt, {
                  dateStyle: "short",
                  timeStyle: "medium",
                })}
              </dd>
              <dt>{t("duration")}</dt>
              <dd className="tabular-nums">{duration(span.durationMs)}</dd>
            </dl>
            <Tabs value={detailTab} onValueChange={value => setDetailTab(String(value))}>
              <TabsList className="max-w-full overflow-x-auto" aria-label={t("spanDetail")}>
                <TabsTrigger value="attributes">{t("attributes")}</TabsTrigger>
                <TabsTrigger value="events">
                  {t("events")} ({formatNumber(span.events.length)})
                </TabsTrigger>
                <TabsTrigger value="links">
                  {t("links")} ({formatNumber(span.links.length)})
                </TabsTrigger>
                <TabsTrigger value="logs">{t("logs")}</TabsTrigger>
              </TabsList>
              <TabsContent value="logs">
                {detailTab === "logs" && <TraceSpanLogs key={`${trace.traceId}:${span.spanId}`} traceId={trace.traceId} workspaceCode={span.attributes["tron.workspace.code"] ?? trace.workspaceCode} span={span} />}
              </TabsContent>
              <TabsContent value="attributes">
                <AttributeList attributes={span.attributes} modelLabel={modelLabel} />
              </TabsContent>
              <TabsContent value="events">
                {span.events.length ? (
                  span.events.map((event, i) => (
                    <div
                      key={i}
                      className="border-b border-[var(--color-outline-variant)] py-3"
                    >
                      <div className="mb-2 flex flex-wrap justify-between gap-2 text-xs">
                        <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">{event.name}</span>
                        <time>
                          {formatDateTime(event.timestamp, {
                            timeStyle: "medium",
                          })}
                        </time>
                      </div>
                      <AttributeList attributes={event.attributes} />
                    </div>
                  ))
                ) : (
                  <p className="py-8 text-center text-sm">{t("empty")}</p>
                )}
              </TabsContent>
              <TabsContent value="links">
                {span.links.length ? (
                  span.links.map((link, i) => (
                    <div
                      key={i}
                      className="space-y-1 border-b border-[var(--color-outline-variant)] py-3 text-xs"
                    >
                      <div className="break-all font-mono">{link.traceId}</div>
                      <Button
                        variant="link"
                        className="h-auto p-0 text-xs"
                        onClick={() => setSelected(link.spanId)}
                        disabled={
                          !detail.spans.some((s) => s.spanId === link.spanId)
                        }
                      >
                        {link.spanId}
                      </Button>
                    </div>
                  ))
                ) : (
                  <p className="py-8 text-center text-sm">{t("empty")}</p>
                )}
              </TabsContent>
            </Tabs>
          </aside>
        )}
      </div>
    </section>
  );
}
function AttributeList({
  attributes,
  modelLabel,
}: {
  attributes: TraceSpan["attributes"];
  modelLabel?: (code: string) => string;
}) {
  const { t } = useTranslation("observability");
  return Object.keys(attributes).length ? (
    <dl className="divide-y divide-[var(--color-outline-variant)] text-xs">
      {Object.entries(attributes).map(([key, value]) => (
        <div key={key} className="space-y-1 py-3">
          <dt className="break-all text-[var(--color-on-surface-variant)]">
            {key}
          </dt>
          <dd className="break-all font-mono">{key === "gen_ai.request.model" && modelLabel ? modelLabel(value) : value}</dd>
        </div>
      ))}
    </dl>
  ) : (
    <p className="py-8 text-center text-sm">{t("empty")}</p>
  );
}
