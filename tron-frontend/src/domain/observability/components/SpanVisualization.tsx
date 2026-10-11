import type { IElementEvent } from "@antv/g6";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import type { TraceDetail } from "../model/Trace";
import { formatTraceDuration } from "./traceFormat";
import { spanGraph } from "./spanGraph";

export function SpanVisualization({
  detail,
  mode,
  selected,
  onSelect,
}: {
  detail: TraceDetail;
  mode: "waterfall" | "topology";
  selected: string;
  onSelect: (id: string) => void;
}) {
  const { t } = useTranslation("observability");
  const { currentLocale } = useLocaleFormatters();
  const container = useRef<HTMLDivElement>(null);
  const callback = useRef(onSelect);
  callback.current = onSelect;
  const data = useMemo(() => spanGraph(detail.spans), [detail.spans]);
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let disposed = false;
    let destroy: (() => void) | undefined;
    setFailed(false);
    const render = async () => {
      if (mode === "waterfall") {
        const { Chart } = await import("@antv/g2");
        if (disposed || !container.current) return;
        const chart = new Chart({
          container: container.current,
          autoFit: true,
          height: Math.max(300, data.rows.length * 40 + 90),
        });
        destroy = () => chart.destroy();
        const rows = data.rows.map(({ span, depth }) => ({
          ...span,
          label: `${"  ".repeat(Math.min(depth, 8))}${span.name}`,
          offset: span.startedAt - detail.trace.startedAt,
          end:
            span.startedAt -
            detail.trace.startedAt +
            Math.max(0, span.durationMs),
          duration: formatTraceDuration(span.durationMs, currentLocale),
        }));
        chart.options({
          type: "interval",
          data: rows,
          coordinate: { transform: [{ type: "transpose" }] },
          encode: { x: "spanId", y: "offset", y1: "end", color: "status" },
          scale: {
            x: { domain: rows.map((row) => row.spanId), padding: 0.4 },
            y: { nice: true },
            color: {
              domain: ["Error", "Completed", "Running", "Unknown"],
              range: ["#d04444", "#16876b", "#3478ce", "#8b8492"],
            },
          },
          axis: {
            x: {
              title: false,
              labelFormatter: (id: string) =>
                rows.find((row) => row.spanId === id)?.label ?? id,
              labelAutoEllipsis: true,
            },
            y: {
              title: t("elapsedTime"),
              labelFormatter: (value: number) =>
                formatTraceDuration(Number(value), currentLocale),
            },
          },
          legend: false,
          style: { radius: 3, minHeight: 3 },
          animate: false,
          tooltip: {
            title: "name",
            items: [
              { field: "spanId", name: "Span ID" },
              { field: "duration", name: t("duration") },
            ],
          },
        });
        chart.on(
          "element:click",
          (event: { data?: { data?: { spanId?: string } } }) => {
            const id = event.data?.data?.spanId;
            if (id) callback.current(id);
          }
        );
        await chart.render();
      } else {
        const { Graph } = await import("@antv/g6");
        if (disposed || !container.current) return;
        const graph = new Graph({
          container: container.current,
          height: Math.max(360, Math.min(650, data.rows.length * 60)),
          autoFit: "view",
          animation: false,
          data: {
            nodes: data.rows.map(({ span }) => ({
              id: span.spanId,
              style: {
                labelText: `${span.name}\n${formatTraceDuration(
                  span.durationMs,
                  currentLocale
                )}`,
                fill: span.status === "Error" ? "#fff0f0" : "#eef7f4",
                stroke: span.status === "Error" ? "#d04444" : "#16876b",
              },
            })),
            edges: data.edges,
          },
          layout: { type: "dagre", rankdir: "TB", nodesep: 32, ranksep: 56 },
          node: {
            type: "rect",
            style: {
              size: [190, 56],
              radius: 8,
              labelPlacement: "center",
              labelFill: "#1c2524",
              labelFontSize: 12,
              labelWordWrap: true,
              labelMaxLines: 2,
              labelLineHeight: 16,
              labelMaxWidth: 174,
            },
          },
          edge: {
            type: "polyline",
            style: { endArrow: true, stroke: "#8b9492" },
          },
          behaviors: ["drag-canvas", "zoom-canvas"],
          padding: 32,
        });
        const observer = new ResizeObserver(() => graph.resize());
        observer.observe(container.current);
        destroy = () => {
          observer.disconnect();
          graph.destroy();
        };
        graph.on("node:click", (event) =>
          callback.current((event as IElementEvent).target.id)
        );
        await graph.render();
      }
    };
    void render().catch(() => {
      if (!disposed) setFailed(true);
    });
    return () => {
      disposed = true;
      destroy?.();
    };
  }, [data, detail.trace.startedAt, mode, currentLocale, t, attempt]);
  return (
    <div className="space-y-3">
      <p className="text-xs text-[var(--color-on-surface-variant)]">
        {t(mode === "topology" ? "topologyHelp" : "waterfallHelp")}
      </p>
      {failed && (
        <div role="alert">
          {t("chartFailed")}
          <Button
            variant="secondary"
            onClick={() => setAttempt((value) => value + 1)}
          >
            {t("retry")}
          </Button>
        </div>
      )}
      <div
        ref={container}
        className="w-full min-w-0"
        role="img"
        aria-label={t(mode)}
      />
      <div className="flex flex-wrap gap-2" aria-label={t("operation")}>
        {data.rows.map(({ span }) => (
          <Button
            key={span.spanId}
            className="h-auto max-w-full py-1 text-xs"
            variant={selected === span.spanId ? "secondary" : "ghost"}
            aria-pressed={selected === span.spanId}
            onClick={() => onSelect(span.spanId)}
          >
            <span className="truncate">{span.name}</span>
            <span>{formatTraceDuration(span.durationMs, currentLocale)}</span>
          </Button>
        ))}
      </div>
    </div>
  );
}
