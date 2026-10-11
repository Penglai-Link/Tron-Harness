import { useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
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
import type { TraceDetail, TraceSpan } from "../model/Trace";
import { TraceStatus } from "./TraceStatus";
import { formatTraceDuration, formatTraceTokens } from "./traceFormat";
import { SpanVisualization } from "./SpanVisualization";
import { traceTree } from "./traceTree";

export function TraceInspector({ detail }: { detail: TraceDetail }) {
  const { t } = useTranslation("observability");
  const { currentLocale, formatNumber, formatDateTime } = useLocaleFormatters();
  const toast = useToast();
  const [selected, setSelected] = useState("");
  const [view, setView] = useState("waterfall");
  const span =
    detail.spans.find((s) => s.spanId === selected) ??
    detail.spans.find((s) => s.name === "agent.run") ??
    detail.spans[0];
  const trace = detail.trace;
  const duration = (value: number) => formatTraceDuration(value, currentLocale);
  const metrics = [
    ["duration", duration(trace.durationMs)],
    ["modelCalls", formatNumber(trace.modelCount)],
    ["toolCalls", formatNumber(trace.toolCount)],
    ["inputTokens", formatTraceTokens(trace.inputTokens, currentLocale)],
    ["outputTokens", formatTraceTokens(trace.outputTokens, currentLocale)],
    ["errors", formatNumber(trace.errorCount)],
  ];
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
          <h2 className="font-semibold">{t("detail")}</h2>
          <code className="break-all text-xs">{trace.traceId}</code>
          <TraceStatus status={trace.status} />
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            aria-label={t("copyTraceId")}
            onClick={() => void copy()}
          >{t("copyTraceId")}</Button>
          <Button
            variant="secondary"
            nativeButton={false}
            render={
              <Link
                to={`/chats?sessionCode=${encodeURIComponent(
                  trace.sessionCode
                )}`}
              />
            }
          >

            {t("openSession")}
          </Button>
          <InfoTooltip ariaLabel={t("help")}>{t("metricsHelp")}</InfoTooltip>
        </div>
      </header>
      <dl className="grid grid-cols-2 gap-5 border-b border-[var(--color-outline-variant)] p-5 sm:grid-cols-3 xl:grid-cols-6">
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
                  {traceTree(detail.spans).map(({ span: item, depth }) => (
                    <TableRow
                      key={item.spanId}
                      data-state={
                        item.spanId === span?.spanId ? "selected" : undefined
                      }
                    >
                      <TableCell>
                        <Button
                          variant="link"
                          className="h-auto max-w-64 justify-start p-0"
                          style={{ paddingLeft: Math.min(depth, 8) * 12 }}
                          onClick={() => setSelected(item.spanId)}
                        >
                          <span className="truncate">
                            {item.attributes["gen_ai.tool.name"] ??
                              item.attributes["gen_ai.request.model"] ??
                              item.name}
                          </span>
                        </Button>
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
                  ))}
                </TableBody>
              </Table>
            </TabsContent>
            <TabsContent value="waterfall">
              {detail.spans.length > 0 && (
                <SpanVisualization
                  detail={detail}
                  mode="waterfall"
                  selected={span?.spanId ?? ""}
                  onSelect={setSelected}
                />
              )}
            </TabsContent>
            <TabsContent value="topology">
              {detail.spans.length > 0 && (
                <SpanVisualization
                  detail={detail}
                  mode="topology"
                  selected={span?.spanId ?? ""}
                  onSelect={setSelected}
                />
              )}
            </TabsContent>
          </Tabs>
          {!detail.spans.length && (
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
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <h3 className="text-sm font-semibold">{span.name}</h3>
              <TraceStatus status={span.status} />
            </div>
            <dl className="mb-5 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 text-xs">
              <dt>Span ID</dt>
              <dd className="break-all font-mono">{span.spanId}</dd>
              <dt>{t("parent")}</dt>
              <dd className="break-all font-mono">
                {span.parentSpanId ?? "—"}
              </dd>
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
            <Tabs defaultValue="attributes">
              <TabsList>
                <TabsTrigger value="attributes">{t("attributes")}</TabsTrigger>
                <TabsTrigger value="events">
                  {t("events")} ({formatNumber(span.events.length)})
                </TabsTrigger>
                <TabsTrigger value="links">
                  {t("links")} ({formatNumber(span.links.length)})
                </TabsTrigger>
              </TabsList>
              <TabsContent value="attributes">
                <AttributeList attributes={span.attributes} />
              </TabsContent>
              <TabsContent value="events">
                {span.events.length ? (
                  span.events.map((event, i) => (
                    <div
                      key={i}
                      className="border-b border-[var(--color-outline-variant)] py-3"
                    >
                      <div className="mb-2 flex flex-wrap justify-between gap-2 text-xs">
                        <span>{event.name}</span>
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
}: {
  attributes: TraceSpan["attributes"];
}) {
  const { t } = useTranslation("observability");
  return Object.keys(attributes).length ? (
    <dl className="divide-y divide-[var(--color-outline-variant)] text-xs">
      {Object.entries(attributes).map(([key, value]) => (
        <div key={key} className="space-y-1 py-3">
          <dt className="break-all text-[var(--color-on-surface-variant)]">
            {key}
          </dt>
          <dd className="break-all font-mono">{value}</dd>
        </div>
      ))}
    </dl>
  ) : (
    <p className="py-8 text-center text-sm">{t("empty")}</p>
  );
}
