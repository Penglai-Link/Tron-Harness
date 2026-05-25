import {
  useMemo,
  useState,
} from "react";
import {
  Gauge,
  Pause,
  RefreshCcw,
  X,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  InspectorPanel,
  MetricCard,
  PageHeader,
  SurfaceCard,
  TablePagination,
  useConsoleShellConfig,
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
        <div className="flex h-full min-w-8 flex-1 flex-col justify-end gap-2" key={item.label}>
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
  const data = useObservabilityOverview();
  const [inspectorTab, setInspectorTab] = useState("Details");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [notice, setNotice] = useState("");
  const tracePagination = useTablePagination(data.traces);

  const rightPanel = useMemo(() => (inspectorOpen ? (
    <InspectorPanel title="Inspector">
      <div className="flex items-center justify-between">
        <Badge tone="success">agent-router-04</Badge>
        <Button size="icon" variant="ghost" aria-label="Close inspector" onClick={() => setInspectorOpen(false)}>
          <X size={16} aria-hidden="true" />
        </Button>
      </div>
      <InspectorTabs tabs={observabilityInspectorTabs} value={inspectorTab} onValueChange={(value) => {
        setInspectorTab(value);
        setNotice(`${value} inspector selected`);
      }} />
      <SurfaceCard className="rounded-xl p-4 shadow-none">
        <p className="text-sm font-semibold">{inspectorTab}</p>
        <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">Router span route.llm.failover is sampled at 100% while the failover controller is in review.</p>
      </SurfaceCard>
      <div className="grid gap-3 text-sm">
        <div className="flex justify-between"><span className="text-[var(--color-on-surface-variant)]">Region</span><span>us-east-1</span></div>
        <div className="flex justify-between"><span className="text-[var(--color-on-surface-variant)]">Budget</span><span>99.95%</span></div>
        <div className="flex justify-between"><span className="text-[var(--color-on-surface-variant)]">Retries</span><span>12</span></div>
      </div>
      <div className="flex gap-2">
        <Button className="flex-1" size="sm" onClick={() => setNotice("Agent paused")}>
          <Pause size={14} aria-hidden="true" />
          Pause Agent
        </Button>
        <Button size="sm" variant="secondary" onClick={() => setNotice("Inspector edit opened")}>Edit</Button>
      </div>
      <Notice value={notice} />
    </InspectorPanel>
  ) : undefined), [inspectorOpen, inspectorTab, notice]);

  useConsoleShellConfig({ rightPanel });

  return (
    <>
      <PageHeader
        title="Observability Dashboard"
        actions={
          <>
            <Button variant="secondary" onClick={() => setInspectorOpen(true)}>Open Inspector</Button>
            <Button onClick={() => setNotice("Trace export queued")}>Export</Button>
          </>
        }
      />
      <div className="grid gap-4 xl:grid-cols-4">
        {data.metrics.map((metric) => <MetricCard key={metric.label} label={metric.label} value={metric.value} trend={metric.trend} icon={<Gauge size={18} aria-hidden="true" />} />)}
      </div>
      <div className="mt-6 grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <SurfaceCard>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-base font-semibold">Request Throughput</h2>
            <Button size="icon" variant="ghost" aria-label="Refresh chart" onClick={() => setNotice("Chart refreshed")}>
              <RefreshCcw size={16} aria-hidden="true" />
            </Button>
          </div>
          <SimpleBarChart data={data.throughput} />
        </SurfaceCard>
        <SurfaceCard>
          <h2 className="text-base font-semibold">Error Heatmap</h2>
          <div className="mt-4 grid grid-cols-6 gap-2">
            {data.heatmap.map((value, index) => (
              <button className="aspect-square rounded-md border border-[var(--color-outline-variant)] text-[11px]" style={{ background: `rgba(47,95,159,${0.15 + value / 120})` }} key={`${value}-${index}`} type="button" onClick={() => setNotice(`Heatmap cell ${index + 1}: ${value} events`)}>
                {value}
              </button>
            ))}
          </div>
        </SurfaceCard>
      </div>
      <DataTable
        title="Recent Traces"
        rows={tracePagination.visibleItems}
        rowKey={(row) => row.id}
        toolbar={<Button size="sm" variant="secondary" onClick={() => setNotice("All traces view opened")}>View All</Button>}
        pagination={<TablePagination itemLabel="traces" {...tracePagination.paginationProps} />}
        columns={[
          { id: "id", header: "Trace", render: (row) => <span className="font-mono text-[13px]">{row.id}</span> },
          { id: "agent", header: "Agent", render: (row) => row.agent },
          { id: "span", header: "Span", render: (row) => <span className="font-mono text-[13px]">{row.span}</span> },
          { id: "latency", header: "Latency", render: (row) => row.latency },
          { id: "status", header: "Status", render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge> },
        ]}
      />
    </>
  );
}
