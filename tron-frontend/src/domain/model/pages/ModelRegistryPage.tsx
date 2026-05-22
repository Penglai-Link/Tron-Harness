import { useState } from "react";
import {
  Plus,
  Search,
  Settings2,
  Sparkles,
  Zap,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SurfaceCard,
  TronShell,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import { useModelList } from "@/domain/model/hooks";
import type { ModelRegistryItem } from "@/domain/model/model";
import { modelVendorFilters } from "@/domain/model/constants";

function RoutePageShell({ activeItem, children }: { activeItem: string; children: React.ReactNode }) {
  return (
    <TronShell activeItem={activeItem} searchPlaceholder="Search Tron...">
      {children}
    </TronShell>
  );
}

function SmallStatus({ children, tone = "neutral" }: { children: React.ReactNode; tone?: "neutral" | "accent" | "success" | "warning" | "error" }) {
  return <Badge tone={tone}>{children}</Badge>;
}

function Notice({ value }: { value: string }) {
  return value ? <p className="sr-only" aria-live="polite">{value}</p> : null;
}

function Meta({ label, value }: { label: string; value: string }) {
  return <div><p className="text-[11px] text-[var(--color-on-surface-variant)]">{label}</p><p className="mt-1 font-medium">{value}</p></div>;
}

function ModelInspector({ model }: { model: ModelRegistryItem }) {
  const [state, setState] = useState<string>(model.status);
  return (
    <SurfaceCard className="p-0">
      <div className="border-b border-[var(--color-outline-variant)] p-5">
        <div className="mb-4 flex items-start justify-between"><span className="flex size-12 items-center justify-center rounded-lg border border-[var(--color-outline-variant)]"><Sparkles size={24} /></span><Button size="icon" variant="ghost" aria-label={`Open ${model.name} settings`} onClick={() => setState("Settings opened")}><Settings2 size={17} /></Button></div>
        <h2 className="text-xl font-semibold">{model.name}</h2>
        <p className="mb-4 mt-1 text-sm text-[var(--color-on-surface-variant)]">{model.vendor} - {model.price}</p>
        <div className="flex gap-2"><Button className="flex-1" variant="secondary" onClick={() => setState("Configure opened")}>Configure</Button><Button className="flex-1" variant="secondary" onClick={() => setState("Disabled")}>Disable</Button></div>
      </div>
      <div className="space-y-6 p-5">
        <section><h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Configuration Details</h3><div className="grid grid-cols-2 gap-4 text-sm"><Meta label="Max Context" value={model.context} /><Meta label="Max Output" value={model.maxOutput} /><Meta label="Training Data" value={model.trainingData} /><Meta label="Inheritance" value={model.inheritance} /></div></section>
        <section><h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Performance Benchmarks</h3><div className="space-y-3">{model.benchmarks.map((benchmark) => <div className="rounded-lg border border-[var(--color-outline-variant)] p-3" key={benchmark.label}><div className="mb-2 flex justify-between text-sm"><span>{benchmark.label}</span><span className="font-semibold">{benchmark.score}%</span></div><div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]"><div className="h-full rounded-full bg-[var(--color-primary)]" style={{ width: `${benchmark.score}%` }} /></div></div>)}</div></section>
        <div className="flex items-center justify-between rounded-lg bg-[var(--color-surface-container-low)] p-3 text-sm"><span className="flex items-center gap-2"><Zap size={16} />Avg. Generation Speed</span><span>{model.speed}</span></div>
        <p className="text-[13px] text-[var(--color-on-surface-variant)]">Panel state: {state}</p>
      </div>
    </SurfaceCard>
  );
}

export function ModelRegistryPage() {
  const models = useModelList();
  const [vendor, setVendor] = useState("All Vendors");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState("gpt-4-turbo");
  const [notice, setNotice] = useState("");
  const visible = models.filter((model) => (vendor === "All Vendors" || model.vendor === vendor) && model.name.toLowerCase().includes(query.toLowerCase()));
  const selected = models.find((model) => model.id === selectedId) ?? models[0];

  return (
    <RoutePageShell activeItem="Models">
      <div className="grid min-h-[740px] gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <section>
          <PageHeader title="Model Registry" description="Manage AI vendors, configurations, and inheritance rules." actions={<Button onClick={() => setNotice("Add model workflow opened")}><Plus size={16} />Add Model</Button>} />
      <Notice value={notice} />
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] pb-4">
            <div className="flex flex-wrap gap-2">
              {modelVendorFilters.map((item) => (
                <button className={cn("rounded-full border px-3 py-1.5 text-sm", vendor === item ? "bg-[var(--color-accent-container)] text-[var(--color-on-accent-container)]" : "border-[var(--color-outline-variant)] bg-white")} type="button" key={item} onClick={() => setVendor(item)}>
                  {item}
                </button>
              ))}
            </div>
            <div className="flex h-9 w-64 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-white px-3">
              <Search size={15} />
              <input className="min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder="Search models..." value={query} onChange={(event) => setQuery(event.target.value)} />
            </div>
          </div>
          <div className="overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white">
            <div className="grid grid-cols-12 gap-4 border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">
              <div className="col-span-4">Model Name</div><div className="col-span-2">Vendor</div><div className="col-span-2">Context</div><div className="col-span-2">Inheritance</div><div className="col-span-2 text-right">Status</div>
            </div>
            {visible.map((model) => (
              <button className={cn("grid w-full grid-cols-12 items-center gap-4 border-b border-[var(--color-outline-variant)] px-4 py-3 text-left last:border-0 hover:bg-[var(--color-hover-surface)]", selected.id === model.id ? "bg-[var(--color-surface-container)]" : "", model.status === "Disabled" ? "opacity-70" : "")} key={model.id} type="button" onClick={() => setSelectedId(model.id)}>
                <div className="col-span-4 flex items-center gap-3"><span className="flex size-8 items-center justify-center rounded border border-[var(--color-outline-variant)]"><Sparkles size={16} /></span><span><span className="block text-sm font-medium">{model.name}</span><span className="block text-[13px] text-[var(--color-on-surface-variant)]">{model.purpose}</span></span></div>
                <div className="col-span-2 text-sm">{model.vendor}</div><div className="col-span-2 font-mono text-[13px]">{model.context}</div><div className="col-span-2 text-sm">{model.inheritance}</div><div className="col-span-2 text-right"><SmallStatus tone={model.status === "Enabled" ? "success" : "neutral"}>{model.status}</SmallStatus></div>
              </button>
            ))}
          </div>
        </section>
        <ModelInspector model={selected} />
      </div>
    </RoutePageShell>
  );
}
