import {
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Gauge,
  Pause,
  RefreshCcw,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/components/ui/button";
import {
  DataTable,
  InspectorPanel,
  MetricCard,
  PageHeader,
  SurfaceCard,
  TablePagination,
  useTabPanelConfig,
  useOpenRightSidebar,
  useTablePagination,
} from "@/shared";

import { useObservabilityOverview } from "@/domain/observability/hooks";
import { cn } from "@/shared/lib/utils";
import {
  getStatusTone,
  observabilityInspectorTabs,
} from "@/domain/observability/constants";


function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function SimpleBarChart({ data }: { data: { label: string; value: number }[] }) {
  const max = Math.max(...data.map((item) => item.value));
  return (
    <div className="flex h-48 items-end gap-3 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
      {data.map((item) => (
        <div className="flex h-full min-w-8 flex-1 flex-col justify-end gap-[12px]" key={item.label}>
          <div className="rounded-t-md bg-[var(--color-primary)]" style={{ height: `${(item.value / max) * 100}%` }} title={`${item.label}: ${item.value}`} />
          <span className="text-center text-[11px] text-[var(--color-on-surface-variant)]">{item.label}</span>
        </div>
      ))}
    </div>
  );
}

function InspectorTabs({ tabs, value, onValueChange }: { tabs: readonly string[]; value: string; onValueChange: (value: string) => void }) {
  return (
    <div className="flex border-b border-[var(--color-outline-variant)]">
      {tabs.map((tab) => {
        const selected = value === tab;
        return (
          <button
            className={cn("border-b-2 px-3 py-2 text-sm font-medium", selected ? "border-[var(--color-primary)] text-[var(--color-on-surface)]" : "border-transparent text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]")}
            type="button"
            key={tab}
            onClick={() => onValueChange(tab)}
          >
            {tab}
          </button>
        );
      })}
    </div>
  );
}

export function ObservabilityPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`observability.${key}`, { defaultValue, ...options })
  );
  const openRightSidebar = useOpenRightSidebar();
  const data = useObservabilityOverview();
  const [inspectorTab, setInspectorTab] = useState("Details");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [notice, setNotice] = useState("");
  const tracePagination = useTablePagination(data.traces);

  const rightPanel = useMemo(() => (inspectorOpen ? (
    <InspectorPanel title={pageT("inspector.title", "Inspector")}>
      <div className="flex items-center justify-between">
        <Badge tone="success">agent-router-04</Badge>
        <Button size="icon" variant="ghost" aria-label={pageT("aria.closeInspector", "Close inspector")} onClick={() => setInspectorOpen(false)}>
          <X size={16} aria-hidden="true" />
        </Button>
      </div>
      <InspectorTabs tabs={observabilityInspectorTabs} value={inspectorTab} onValueChange={(value) => {
        setInspectorTab(value);
        setNotice(pageT("messages.tabSelected", "{{tab}} inspector selected", { tab: value }));
      }} />
      <SurfaceCard className="rounded-xl p-4 shadow-none">
        <p className="text-sm font-semibold">{inspectorTab}</p>
        <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">{pageT("inspector.description", "Router span route.llm.failover is sampled at 100% while the failover controller is in review.")}</p>
      </SurfaceCard>
      <div className="grid gap-3 text-sm">
        <div className="flex justify-between"><span className="text-[var(--color-on-surface-variant)]">{pageT("fields.region", "Region")}</span><span>us-east-1</span></div>
        <div className="flex justify-between"><span className="text-[var(--color-on-surface-variant)]">{pageT("fields.budget", "Budget")}</span><span>99.95%</span></div>
        <div className="flex justify-between"><span className="text-[var(--color-on-surface-variant)]">{pageT("fields.retries", "Retries")}</span><span>12</span></div>
      </div>
      <div className="flex gap-[12px]">
        <Button className="flex-1" size="sm" onClick={() => setNotice(pageT("messages.agentPaused", "Agent paused"))}>
          {pageT("actions.pauseAgent", "Pause Agent")}
        </Button>
        <Button size="sm" variant="secondary" onClick={() => setNotice(pageT("messages.editOpened", "Inspector edit opened"))}>{pageT("actions.edit", "Edit")}</Button>
      </div>
      <Notice value={notice} />
    </InspectorPanel>
  ) : null), [inspectorOpen, inspectorTab, notice]);

  useTabPanelConfig({ rightPanel });

  function openInspector() {
    setInspectorOpen(true);
    openRightSidebar();
  }

  return (
    <>
      <PageHeader
        title={pageT("title", "Observability Dashboard")}
        actions={
          <>
            <Button size="lg" variant="outline" onClick={openInspector}>{pageT("actions.openInspector", "Open Inspector")}</Button>
            <Button size="lg" onClick={() => setNotice(pageT("messages.exportQueued", "Trace export queued"))}>{pageT("actions.export", "Export")}</Button>
          </>
        }
      />
      <div className="grid gap-4 xl:grid-cols-4">
        {data.metrics.map((metric) => <MetricCard key={metric.label} label={metric.label} value={metric.value} trend={metric.trend} icon={<Gauge size={18} aria-hidden="true" />} />)}
      </div>
      <div className="grid mb-6 gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <SurfaceCard>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-base font-semibold">{pageT("charts.requestThroughput", "Request Throughput")}</h2>
            <Button size="icon" variant="ghost" aria-label={pageT("aria.refreshChart", "Refresh chart")} onClick={() => setNotice(pageT("messages.chartRefreshed", "Chart refreshed"))}>
              <RefreshCcw size={16} aria-hidden="true" />
            </Button>
          </div>
          <SimpleBarChart data={data.throughput} />
        </SurfaceCard>
        <SurfaceCard>
          <h2 className="text-base font-semibold">{pageT("charts.errorHeatmap", "Error Heatmap")}</h2>
          <div className="mt-4 grid grid-cols-6 gap-[12px]">
            {data.heatmap.map((value, index) => (
              <button className="aspect-square rounded-md border border-[var(--color-outline-variant)] text-[11px]" style={{ background: `rgba(47,95,159,${0.15 + value / 120})` }} key={`${value}-${index}`} type="button" onClick={() => setNotice(pageT("messages.heatmapCell", "Heatmap cell {{index}}: {{value}} events", { index: index + 1, value }))}>
                {value}
              </button>
            ))}
          </div>
        </SurfaceCard>
      </div>
      <DataTable
        title={pageT("traces.title", "Recent Traces")}
        rows={tracePagination.visibleItems}
        rowKey={(row) => row.id}
        toolbar={<Button size="lg" variant="outline" onClick={() => setNotice(pageT("messages.allTracesOpened", "All traces view opened"))}>{pageT("actions.viewAll", "View All")}</Button>}
        pagination={<TablePagination itemLabel={pageT("traces.itemLabel", "traces")} {...tracePagination.paginationProps} />}
        columns={[
          { id: "id", header: pageT("columns.trace", "Trace"), render: (row) => <span className="font-mono text-[13px]">{row.id}</span> },
          { id: "agent", header: pageT("columns.agent", "Agent"), render: (row) => row.agent },
          { id: "span", header: pageT("columns.span", "Span"), render: (row) => <span className="font-mono text-[13px]">{row.span}</span> },
          { id: "latency", header: pageT("columns.latency", "Latency"), render: (row) => row.latency },
          { id: "status", header: pageT("columns.status", "Status"), render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge> },
        ]}
      />
    </>
  );
}
