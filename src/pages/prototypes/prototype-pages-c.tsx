import { useMemo, useState, type ReactNode } from "react";
import {
  Activity,
  AlertCircle,
  BarChart3,
  Bell,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleUserRound,
  Copy,
  Database,
  Eye,
  EyeOff,
  FileArchive,
  Filter,
  FolderOpen,
  Gauge,
  HelpCircle,
  Image,
  KeyRound,
  LayoutDashboard,
  LifeBuoy,
  MoreHorizontal,
  Pause,
  Plus,
  RefreshCcw,
  Search,
  ShieldCheck,
  SlidersHorizontal,
  Settings2,
  Terminal,
  Upload,
  UserPlus,
  Workflow,
  X,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable, Field, InspectorPanel, MetricCard, PageHeader, SegmentedTabs, SelectInput, SurfaceCard, TextArea, TextInput, ToggleSwitch, TronShell } from "@/components/tron";
import {
  getTaskDetail,
  listCapabilities,
  listKanbanColumns,
  listObservabilityData,
  listPermissionDomains,
  listPrompts,
  listResources,
  listRoles,
  listUsers,
  listVariables,
  type CapabilityRecord,
  type KanbanTask,
  type PromptRecord,
  type ResourceRecord,
  type StatusTone,
  type UserRecord,
  type VariableRecord,
} from "@/services/prototypes/prototype-data-c";
import { cn } from "@/lib/utils";

type PrototypeComponent = () => ReactNode;

const badgeToneByStatus = (status: string): StatusTone => {
  const normalized = status.toLowerCase();
  if (normalized.includes("healthy") || normalized.includes("active") || normalized.includes("connected") || normalized.includes("completed")) return "success";
  if (normalized.includes("review") || normalized.includes("sync") || normalized.includes("progress") || normalized.includes("override")) return "warning";
  if (normalized.includes("suspend") || normalized.includes("disabled") || normalized.includes("deprecated")) return "error";
  return "neutral";
};

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function ProgressBar({ value }: { value: number }) {
  return (
    <div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
      <div className="h-full rounded-full bg-[var(--color-primary)]" style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
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

function InspectorTabs({ tabs, value, onValueChange }: { tabs: string[]; value: string; onValueChange: (value: string) => void }) {
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
  const data = useMemo(() => listObservabilityData(), []);
  const [inspectorTab, setInspectorTab] = useState("Details");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [notice, setNotice] = useState("Live trace stream active");

  const rightPanel = inspectorOpen ? (
    <InspectorPanel title="Inspector">
      <div className="flex items-center justify-between">
        <Badge tone="success">agent-router-04</Badge>
        <Button size="icon" variant="ghost" aria-label="Close inspector" onClick={() => setInspectorOpen(false)}>
          <X size={16} aria-hidden="true" />
        </Button>
      </div>
      <InspectorTabs tabs={["Details", "Logs", "Traces", "Health"]} value={inspectorTab} onValueChange={(value) => {
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
  ) : undefined;

  return (
    <TronShell activeItem="Observability" searchPlaceholder="Search traces, agents, metrics..." rightPanel={rightPanel}>
      <PageHeader
        title="Observability Dashboard"
        eyebrow="Runtime telemetry"
        status="Live"
        description="Trace volume, latency, error budgets, heatmaps, and sampled agent spans."
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
        rows={data.traces}
        rowKey={(row) => row.id}
        toolbar={<Button size="sm" variant="secondary" onClick={() => setNotice("All traces view opened")}>View All</Button>}
        columns={[
          { id: "id", header: "Trace", render: (row) => <span className="font-mono text-[13px]">{row.id}</span> },
          { id: "agent", header: "Agent", render: (row) => row.agent },
          { id: "span", header: "Span", render: (row) => <span className="font-mono text-[13px]">{row.span}</span> },
          { id: "latency", header: "Latency", render: (row) => row.latency },
          { id: "status", header: "Status", render: (row) => <Badge tone={badgeToneByStatus(row.status)}>{row.status}</Badge> },
        ]}
      />
    </TronShell>
  );
}

export function PromptsPage() {
  const prompts = useMemo(() => listPrompts(), []);
  const [category, setCategory] = useState("All");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(prompts[0]?.id ?? "");
  const [notice, setNotice] = useState("Prompt details loaded");
  const selected = prompts.find((prompt) => prompt.id === selectedId) ?? prompts[0];
  const filtered = prompts.filter((prompt) => (category === "All" || prompt.category === category) && prompt.name.toLowerCase().includes(query.toLowerCase()));

  return (
    <TronShell activeItem="Prompts" searchPlaceholder="Search anything...">
      <PageHeader
        title="Prompt Management"
        status="Split pane"
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice(`${filtered.length} prompts exported`)}>Bulk Export</Button>
            <Button onClick={() => setNotice("New prompt flow opened")}><Plus size={16} aria-hidden="true" />New Prompt</Button>
          </>
        }
      />
      <div className="grid min-h-[760px] gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <SurfaceCard className="p-0">
          <div className="border-b border-[var(--color-outline-variant)] px-5 pt-4">
            <div className="flex flex-wrap gap-1">
              {["All", "System", "Persona", "Tools", "Templates"].map((item) => (
                <button className={cn("border-b-2 px-4 py-2 text-sm", category === item ? "border-[var(--color-primary)] font-medium" : "border-transparent text-[var(--color-on-surface-variant)]")} type="button" key={item} onClick={() => setCategory(item)}>
                  {item === "All" ? "All Prompts" : item}
                </button>
              ))}
            </div>
            <div className="flex flex-wrap items-center gap-2 py-4">
              <SearchField placeholder="Search prompts..." value={query} onChange={setQuery} />
              <Button size="sm" variant="secondary" onClick={() => setNotice("Prompt filters opened")}><Filter size={14} aria-hidden="true" />Filter</Button>
            </div>
          </div>
          <DataTable
            rows={filtered}
            rowKey={(row) => row.id}
            pagination={<span>{filtered.length} prompts visible</span>}
            columns={[
              { id: "name", header: "Name", render: (row) => <button className="font-medium text-[var(--color-primary)]" type="button" onClick={() => setSelectedId(row.id)}>{row.name}</button> },
              { id: "category", header: "Category", render: (row) => <Badge>{row.category}</Badge> },
              { id: "inheritance", header: "Inheritance", render: (row) => row.inheritance },
              { id: "updated", header: "Last Updated", render: (row) => row.updated },
              { id: "actions", header: "Actions", align: "right", render: (row) => <Button size="icon" variant="ghost" aria-label={`Select ${row.name}`} onClick={() => setSelectedId(row.id)}><MoreHorizontal size={16} aria-hidden="true" /></Button> },
            ]}
          />
        </SurfaceCard>
        <PromptInspector prompt={selected} notice={notice} setNotice={setNotice} />
      </div>
    </TronShell>
  );
}

function PromptInspector({ prompt, notice, setNotice }: { prompt: PromptRecord; notice: string; setNotice: (value: string) => void }) {
  const [variables, setVariables] = useState(prompt.variables);
  return (
    <SurfaceCard>
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-xl font-semibold">Prompt Details</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{prompt.name}</p>
        </div>
        <Button size="icon" variant="ghost" aria-label="Close prompt details" onClick={() => setNotice("Prompt detail panel closed")}><X size={16} aria-hidden="true" /></Button>
      </div>
      <div className="mt-5 rounded-lg bg-[var(--color-surface-container-low)] p-4">
        <pre className="whitespace-pre-wrap text-[13px] leading-6">{prompt.body}</pre>
      </div>
      <div className="mt-5 flex justify-end">
        <Button size="sm" variant="secondary" onClick={() => setNotice("Prompt body copied")}><Copy size={14} aria-hidden="true" />Copy</Button>
      </div>
      <div className="mt-6 grid gap-3">
        <h3 className="text-sm font-semibold">Test Variables</h3>
        {Object.entries(variables).map(([key, value]) => (
          <Field label={key} key={key}>
            {key === "user_query" ? (
              <TextArea value={value} onChange={(event) => setVariables((current) => ({ ...current, [key]: event.target.value }))} />
            ) : (
              <TextInput value={value} onChange={(event) => setVariables((current) => ({ ...current, [key]: event.target.value }))} />
            )}
          </Field>
        ))}
        <Button onClick={() => setNotice(`Preview ran for ${prompt.name}`)}>Run Test Preview</Button>
        <Notice value={notice} />
      </div>
    </SurfaceCard>
  );
}

export function ResourcesPage() {
  const resources = useMemo(() => listResources(), []);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [typeFilter, setTypeFilter] = useState("All");
  const [statusFilter, setStatusFilter] = useState("Active");
  const [page, setPage] = useState(1);
  const [rowMenu, setRowMenu] = useState<string | null>(null);
  const [agentOpen, setAgentOpen] = useState(false);
  const [notice, setNotice] = useState("Resource inventory ready");
  const filtered = resources.filter((resource) => {
    const matchesSearch = resource.name.toLowerCase().includes(query.toLowerCase()) || resource.type.toLowerCase().includes(query.toLowerCase()) || resource.location.toLowerCase().includes(query.toLowerCase());
    const matchesType = typeFilter === "All" || resource.type === typeFilter;
    return matchesSearch && matchesType;
  });
  const allSelected = filtered.length > 0 && filtered.every((resource) => selected.includes(resource.id));
  const toggleSelected = (id: string) => setSelected((current) => (current.includes(id) ? current.filter((item) => item !== id) : [...current, id]));
  const toggleAllSelected = () => {
    setSelected((current) => (allSelected ? current.filter((id) => !filtered.some((resource) => resource.id === id)) : Array.from(new Set([...current, ...filtered.map((resource) => resource.id)]))));
    setNotice(allSelected ? "Visible resources cleared" : "Visible resources selected");
  };
  const cycleStatusFilter = () => {
    const states = ["Active", "Inherited", "Deprecated"];
    const next = states[(states.indexOf(statusFilter) + 1) % states.length];
    setStatusFilter(next);
    setNotice(`Status filter changed to ${next}`);
  };
  const cycleTypeFilter = () => {
    const states = ["All", "PostgreSQL 14", "AWS S3 Bucket", "JSON Config File", "Media Library", "Redis 5.0"];
    const next = states[(states.indexOf(typeFilter) + 1) % states.length];
    setTypeFilter(next);
    setNotice(`Type filter changed to ${next}`);
  };

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <aside className="fixed bottom-0 left-0 top-0 z-50 hidden w-[280px] flex-col border-r border-[var(--color-outline-variant)] bg-white p-5 lg:flex">
        <a className="mb-14 flex items-center gap-3" href="/dashboard" aria-label="Tron Console dashboard">
          <span className="flex size-10 items-center justify-center rounded-lg bg-[var(--color-primary)] text-[var(--color-on-primary)]">
            <Database size={18} aria-hidden="true" />
          </span>
          <span>
            <span className="block text-xl font-semibold leading-tight">Tron Console</span>
            <span className="block text-base text-[var(--color-on-surface-variant)]">Enterprise Management</span>
          </span>
        </a>

        <nav className="flex flex-1 flex-col gap-3" aria-label="Enterprise management navigation">
          {[
            { label: "Dashboard", icon: LayoutDashboard, href: "/dashboard" },
            { label: "Resources", icon: FolderOpen, href: "/resources" },
            { label: "Workflows", icon: Workflow, href: "/automations" },
            { label: "Analytics", icon: BarChart3, href: "/observability" },
            { label: "Settings", icon: Settings2, href: "/admin/workspace" },
          ].map((item) => {
            const Icon = item.icon;
            const active = item.label === "Resources";
            return (
              <a
                className={cn(
                  "flex h-11 items-center gap-3 rounded-lg px-4 text-base transition-colors",
                  active ? "bg-[var(--color-surface-container-high)] font-semibold text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface-container-low)] hover:text-[var(--color-on-surface)]",
                )}
                aria-current={active ? "page" : undefined}
                href={item.href}
                key={item.label}
              >
                <Icon size={22} aria-hidden="true" />
                {item.label}
              </a>
            );
          })}
        </nav>

        <div className="border-t border-[var(--color-outline-variant)] pt-5">
          <Button className="w-full" onClick={() => setNotice("New asset workflow opened")}>
            <Plus size={17} aria-hidden="true" />
            New Asset
          </Button>
        </div>
      </aside>

      <div className="min-h-screen lg:ml-[280px]">
        <header className="sticky top-0 z-40 flex h-14 items-center justify-between border-b border-[var(--color-outline-variant)] bg-white/85 px-6 backdrop-blur">
          <div className="flex min-w-0 flex-1 items-center gap-6">
            <h2 className="shrink-0 text-sm font-medium">Resources Management</h2>
            <div className="relative w-full max-w-[320px]">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={18} aria-hidden="true" />
              <input
                className="h-10 w-full rounded-lg border border-transparent bg-[var(--color-surface-container-low)] pl-10 pr-3 text-sm outline-none transition-colors focus:border-[var(--color-outline-variant)] focus:bg-white"
                placeholder="Search resources..."
                type="search"
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value);
                  setNotice(event.target.value ? `Search filtered to ${event.target.value}` : "Search cleared");
                }}
              />
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button size="icon" variant="ghost" aria-label="View notifications" onClick={() => setNotice("Notifications panel opened")}><Bell size={20} aria-hidden="true" /></Button>
            <Button size="icon" variant="ghost" aria-label="Open help" onClick={() => setNotice("Help center opened")}><HelpCircle size={20} aria-hidden="true" /></Button>
            <Button size="icon" variant="ghost" aria-label="Open account menu" onClick={() => setNotice("Account menu opened")}><CircleUserRound size={22} aria-hidden="true" /></Button>
          </div>
        </header>

        <main className="flex min-h-[calc(100vh-3.5rem)] flex-col gap-6 px-6 py-8">
          <section className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <h1 className="text-3xl font-semibold leading-tight">Data &amp; Assets</h1>
              <p className="mt-2 text-base text-[var(--color-on-surface-variant)]">Manage all connected data sources, files, and visual assets.</p>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="secondary" onClick={() => setNotice(`${selected.length} resources queued for bulk actions`)}>
                <SlidersHorizontal size={17} aria-hidden="true" />
                Bulk Actions
              </Button>
              <Button onClick={() => setNotice("Import Resource drawer opened")}>
                <Upload size={17} aria-hidden="true" />
                Import Resource
              </Button>
            </div>
          </section>

          <section className="grid gap-5 xl:grid-cols-3" aria-label="Resource metrics">
            <EnterpriseMetric icon={<Database size={18} aria-hidden="true" />} label="Active Data Sources" value="24" detail="+2 this week" />
            <EnterpriseMetric icon={<FileArchive size={18} aria-hidden="true" />} label="Managed Files" value="1,492" detail="Across 8 environments" />
            <EnterpriseMetric icon={<RefreshCcw size={18} aria-hidden="true" />} label="Total Storage" value="8.4 TB" detail="72% of allocated capacity" />
          </section>

          <section className="flex min-h-[640px] flex-1 flex-col overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white shadow-[0_2px_8px_rgba(0,0,0,0.04)]">
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3">
              <div className="flex flex-wrap items-center gap-2">
                <button className="inline-flex h-10 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 text-sm hover:bg-[var(--color-surface-container)]" type="button" onClick={cycleTypeFilter}>
                  Type: {typeFilter}
                  <ChevronDown size={16} aria-hidden="true" />
                </button>
                <button className="inline-flex h-10 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 text-sm hover:bg-[var(--color-surface-container)]" type="button" onClick={cycleStatusFilter}>
                  Status: {statusFilter}
                  <ChevronDown size={16} aria-hidden="true" />
                </button>
              </div>
              <Button size="icon" variant="ghost" aria-label="Open advanced resource filters" onClick={() => setNotice("Advanced filters opened")}>
                <Filter size={18} aria-hidden="true" />
              </Button>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full min-w-[860px] border-collapse text-left">
                <thead>
                  <tr className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-bright)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                    <th className="w-14 px-5 py-3">
                      <input className="size-4 rounded border-[var(--color-outline-variant)]" checked={allSelected} aria-label="Select all resources" type="checkbox" onChange={toggleAllSelected} />
                    </th>
                    <th className="px-5 py-3">Resource Name</th>
                    <th className="px-5 py-3">Type</th>
                    <th className="px-5 py-3">Status</th>
                    <th className="px-5 py-3">Location</th>
                    <th className="px-5 py-3 text-right">Last Accessed</th>
                    <th className="w-12 px-5 py-3" />
                  </tr>
                </thead>
                <tbody className="text-[15px]">
                  {filtered.map((resource) => (
                    <tr className="group border-b border-[var(--color-outline-variant)] last:border-0 hover:bg-[var(--color-surface-container-low)]" key={resource.id}>
                      <td className="px-5 py-4">
                        <input className="size-4 rounded border-[var(--color-outline-variant)]" checked={selected.includes(resource.id)} aria-label={`Select ${resource.name}`} type="checkbox" onChange={() => toggleSelected(resource.id)} />
                      </td>
                      <td className="px-5 py-4">
                        <div className="flex items-center gap-3">
                          <ResourceIcon resource={resource} />
                          <span className="font-medium">{resource.name}</span>
                        </div>
                      </td>
                      <td className="px-5 py-4 text-[var(--color-on-surface-variant)]">{resource.type}</td>
                      <td className="px-5 py-4"><Badge tone={badgeToneByStatus(resource.status)}>{resource.status}</Badge></td>
                      <td className="px-5 py-4 text-[var(--color-on-surface-variant)]">{resource.location}</td>
                      <td className="px-5 py-4 text-right text-[var(--color-on-surface-variant)]">{resource.lastAccessed}</td>
                      <td className="relative px-5 py-4 text-right">
                        <Button size="icon" variant="ghost" aria-label={`Open actions for ${resource.name}`} onClick={() => setRowMenu((current) => (current === resource.id ? null : resource.id))}>
                          <MoreHorizontal size={16} aria-hidden="true" />
                        </Button>
                        {rowMenu === resource.id ? (
                          <div className="absolute right-4 top-12 z-20 w-36 rounded-lg border border-[var(--color-outline-variant)] bg-white p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.12)]">
                            <button className="block w-full rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container-low)]" type="button" onClick={() => setNotice(`${resource.name} details opened`)}>Open details</button>
                            <button className="block w-full rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container-low)]" type="button" onClick={() => setNotice(`${resource.name} policy review queued`)}>Review policy</button>
                          </div>
                        ) : null}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <div className="mt-auto flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-bright)] px-5 py-3 text-sm text-[var(--color-on-surface-variant)]">
              <span>Showing 1 to {filtered.length} of 1,516 resources</span>
              <div className="flex items-center gap-1">
                <Button size="icon" variant="ghost" disabled={page === 1} aria-label="Previous resources page" onClick={() => setPage((current) => Math.max(1, current - 1))}><ChevronLeft size={18} aria-hidden="true" /></Button>
                {[1, 2, 3].map((item) => (
                  <button
                    className={cn("flex size-8 items-center justify-center rounded-lg text-sm", page === item ? "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface-container)]")}
                    type="button"
                    key={item}
                    onClick={() => {
                      setPage(item);
                      setNotice(`Resources page ${item} selected`);
                    }}
                  >
                    {item}
                  </button>
                ))}
                <Button size="icon" variant="ghost" aria-label="Next resources page" onClick={() => {
                  setPage((current) => Math.min(3, current + 1));
                  setNotice("Next resources page selected");
                }}><ChevronRight size={18} aria-hidden="true" /></Button>
              </div>
            </div>
          </section>

          <p className="sr-only" aria-live="polite">{notice}</p>
        </main>
      </div>

      <button
        className="fixed bottom-6 right-6 z-50 flex size-14 items-center justify-center rounded-full bg-[var(--color-primary)] text-[var(--color-on-primary)] shadow-[0_8px_24px_rgba(0,0,0,0.16)] transition-transform hover:scale-105 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
        type="button"
        aria-expanded={agentOpen}
        aria-label="Open support agent"
        onClick={() => setAgentOpen((current) => !current)}
      >
        <LifeBuoy size={26} aria-hidden="true" />
      </button>
      {agentOpen ? (
        <section className="fixed bottom-24 right-6 z-50 w-80 rounded-xl border border-[var(--color-outline-variant)] bg-white p-4 shadow-[0_16px_42px_rgba(0,0,0,0.14)]">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="font-semibold">Support Agent</p>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">Resource import, access, and storage help is available.</p>
            </div>
            <Button size="icon" variant="ghost" aria-label="Close support agent" onClick={() => setAgentOpen(false)}><X size={16} aria-hidden="true" /></Button>
          </div>
        </section>
      ) : null}
    </div>
  );
}

function EnterpriseMetric({ detail, icon, label, value }: { detail: string; icon: ReactNode; label: string; value: string }) {
  return (
    <section className="rounded-xl border border-[var(--color-outline-variant)] bg-white p-6 shadow-[0_2px_8px_rgba(0,0,0,0.04)]">
      <div className="flex items-center gap-3 text-base text-[var(--color-on-surface-variant)]">
        {icon}
        <span>{label}</span>
      </div>
      <p className="mt-4 text-5xl font-semibold leading-none">{value}</p>
      <p className="mt-4 text-sm text-[var(--color-on-surface-variant)]">{detail}</p>
    </section>
  );
}

function ResourceIcon({ resource }: { resource: ResourceRecord }) {
  const iconClassName = "size-4";
  if (resource.type.includes("PostgreSQL")) return <Database className={iconClassName} aria-hidden="true" />;
  if (resource.type.includes("S3")) return <FolderOpen className={iconClassName} aria-hidden="true" />;
  if (resource.type.includes("JSON")) return <Settings2 className={iconClassName} aria-hidden="true" />;
  if (resource.type.includes("Media")) return <Image className={iconClassName} aria-hidden="true" />;
  return <AlertCircle className={iconClassName} aria-hidden="true" />;
}

export function RoleMatrixPage() {
  const roles = useMemo(() => listRoles(), []);
  const domains = useMemo(() => listPermissionDomains(), []);
  const [activeTab, setActiveTab] = useState("roles");
  const [query, setQuery] = useState("");
  const [overrides, setOverrides] = useState<Record<string, string[]>>({});
  const [notice, setNotice] = useState("Role matrix ready.");
  const filteredRoles = roles.filter((role) => role.name.toLowerCase().includes(query.toLowerCase()));

  function togglePermission(roleId: string, domain: string, code: string) {
    const key = `${roleId}:${domain}`;
    const base = roles.find((role) => role.id === roleId)?.permissions[domain] ?? [];
    const current = overrides[key] ?? base;
    setOverrides((values) => ({ ...values, [key]: current.includes(code) ? current.filter((item) => item !== code) : [...current, code] }));
  }

  return (
    <TronShell activeItem="Roles" searchPlaceholder="Search users or roles...">
      <PageHeader
        title="Users & Roles"
        status="Permission grid"
        actions={<Button onClick={() => setNotice("Custom role editor opened")}><Plus size={16} aria-hidden="true" />Add Custom Role</Button>}
      />
      <SurfaceCard className="mb-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <SegmentedTabs value={activeTab} onValueChange={setActiveTab} items={[{ label: "User Directory", value: "users" }, { label: "Role Matrix", value: "roles" }, { label: "Audit Logs", value: "audit" }]} />
          <div className="flex items-center gap-2">
            <SearchField placeholder="Search roles..." value={query} onChange={setQuery} />
            <Button size="sm" variant="secondary" onClick={() => setNotice(`Filter drawer opened for ${filteredRoles.length} roles`)}><Filter size={14} aria-hidden="true" />Filter</Button>
          </div>
        </div>
      </SurfaceCard>
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <SurfaceCard className="overflow-hidden p-0">
        <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] p-5">
          <h2 className="text-xl font-semibold">Platform Permissions</h2>
          <Badge tone="accent">{activeTab === "roles" ? "Role Matrix" : "Related tab selected"}</Badge>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[980px] border-collapse text-left text-sm">
            <thead>
              <tr className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]">
                <th className="sticky left-0 z-10 w-48 border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[11px] uppercase text-[var(--color-on-surface-variant)]">Role Name</th>
                {domains.map((domain) => <th className="border-r border-[var(--color-outline-variant)] p-4 text-center text-[11px] uppercase text-[var(--color-on-surface-variant)]" key={domain}>{domain}</th>)}
              </tr>
            </thead>
            <tbody>
              {filteredRoles.map((role) => (
                <tr className="border-b border-[var(--color-outline-variant)] hover:bg-[var(--color-hover-surface)]" key={role.id}>
                  <td className="sticky left-0 border-r border-[var(--color-outline-variant)] bg-white p-4 font-medium">
                    {role.name}
                    <div className="mt-1 text-[11px] text-[var(--color-on-surface-variant)]">{role.description}</div>
                  </td>
                  {domains.map((domain) => {
                    const permissions = overrides[`${role.id}:${domain}`] ?? role.permissions[domain] ?? [];
                    return (
                      <td className="border-r border-[var(--color-outline-variant)] p-4" key={domain}>
                        <div className="flex justify-center gap-1">
                          {["R", "W", "D", "A"].map((code) => {
                            const active = permissions.includes(code);
                            return (
                              <button className={cn("flex size-6 items-center justify-center rounded text-[10px] font-semibold", active ? "bg-[var(--color-primary)] text-[var(--color-on-primary)]" : "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)] opacity-60")} type="button" key={code} title={code} onClick={() => togglePermission(role.id, domain, code)}>
                                {code}
                              </button>
                            );
                          })}
                        </div>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="flex flex-wrap items-center gap-4 border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[13px] text-[var(--color-on-surface-variant)]">
          {["R Read", "W Write", "D Delete", "A Admin"].map((item) => <span key={item}>{item}</span>)}
        </div>
      </SurfaceCard>
    </TronShell>
  );
}

export function SkillsPage() {
  const [mode, setMode] = useState<"skills" | "mcp">("skills");
  const [query, setQuery] = useState("");
  const [capabilities, setCapabilities] = useState(() => listCapabilities());
  const [notice, setNotice] = useState("Capability directory ready.");
  const filtered = capabilities.filter((item) => item.mode === mode && item.name.toLowerCase().includes(query.toLowerCase()));
  const [selectedId, setSelectedId] = useState(filtered[0]?.id ?? "python");
  const selected = capabilities.find((item) => item.id === selectedId) ?? filtered[0] ?? capabilities[0];

  function toggleCapability(id: string, checked: boolean) {
    setCapabilities((items) => items.map((item) => (item.id === id ? { ...item, enabled: checked, status: checked ? "Connected" : "Disabled" } : item)));
  }

  return (
    <TronShell activeItem="Skills" searchPlaceholder="Search capabilities...">
      <PageHeader
        title="Capabilities Management"
        description="Configure agent skills and external MCP server connections."
        actions={
          <>
            <SegmentedTabs value={mode} onValueChange={(value) => setMode(value as "skills" | "mcp")} items={[{ label: "Skills", value: "skills" }, { label: "MCP Servers", value: "mcp" }]} />
            <Button onClick={() => setNotice("New skill workflow opened")}><Plus size={16} aria-hidden="true" />New Skill</Button>
          </>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <div className="grid min-h-[760px] gap-4 xl:grid-cols-[minmax(0,1fr)_420px]">
        <SurfaceCard className="p-0">
          <div className="border-b border-[var(--color-outline-variant)] p-4">
            <SearchField placeholder="Search skills and servers..." value={query} onChange={setQuery} />
          </div>
          <div className="space-y-1 p-2">
            {filtered.map((item) => <CapabilityRow key={item.id} item={item} selected={selected.id === item.id} onSelect={() => setSelectedId(item.id)} onToggle={toggleCapability} />)}
          </div>
        </SurfaceCard>
        <CapabilityInspector capability={selected} />
      </div>
    </TronShell>
  );
}

function CapabilityRow({ item, selected, onSelect, onToggle }: { item: CapabilityRecord; selected: boolean; onSelect: () => void; onToggle: (id: string, checked: boolean) => void }) {
  return (
    <button className={cn("flex w-full items-center justify-between rounded-lg border p-3 text-left", selected ? "border-[var(--color-outline-variant)] bg-[var(--color-surface-container)]" : "border-transparent hover:bg-[var(--color-surface-container-low)]")} type="button" onClick={onSelect}>
      <div className="flex items-start gap-3">
        <span className="flex size-10 items-center justify-center rounded-lg border bg-white text-[var(--color-primary)]"><Terminal size={18} aria-hidden="true" /></span>
        <span>
          <span className="block text-sm font-medium">{item.name}</span>
          <span className="mt-1 block text-[13px] text-[var(--color-on-surface-variant)]">{item.description}</span>
        </span>
      </div>
      <ToggleSwitch checked={item.enabled} label="" onCheckedChange={(checked) => onToggle(item.id, checked)} />
    </button>
  );
}

function CapabilityInspector({ capability }: { capability: CapabilityRecord }) {
  const [notice, setNotice] = useState("Schema ready");
  return (
    <SurfaceCard>
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold">{capability.name}</h2>
          <div className="mt-2 flex items-center gap-2"><span className="size-2 rounded-full bg-[var(--color-success)]" /><Badge tone={badgeToneByStatus(capability.status)}>{capability.status}</Badge></div>
        </div>
        <Button size="icon" variant="ghost" aria-label="Capability actions" onClick={() => setNotice("Capability menu opened")}><MoreHorizontal size={16} aria-hidden="true" /></Button>
      </div>
      <div className="mt-6 grid grid-cols-2 gap-4 border-y border-[var(--color-outline-variant)] py-4 text-sm">
        <div><span className="block text-[11px] uppercase text-[var(--color-on-surface-variant)]">Provider</span><span className="font-medium">{capability.provider}</span></div>
        <div><span className="block text-[11px] uppercase text-[var(--color-on-surface-variant)]">Latency</span><span className="font-medium">{capability.latency}</span></div>
      </div>
      <p className="mt-5 text-sm leading-6 text-[var(--color-on-surface-variant)]">{capability.description}</p>
      <div className="mt-5">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Tool Definition (Schema)</h3>
          <Button size="sm" variant="ghost" onClick={() => setNotice("Schema copied")}><Copy size={14} aria-hidden="true" />Copy</Button>
        </div>
        <pre className="overflow-x-auto rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-[12px] leading-5">{capability.schema}</pre>
      </div>
      <div className="mt-5 space-y-2">
        <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Recent Activity</h3>
        {capability.activity.map((event) => <div className="flex items-center gap-2 text-sm text-[var(--color-on-surface-variant)]" key={event}><Check size={14} aria-hidden="true" />{event}</div>)}
      </div>
      <Notice value={notice} />
    </SurfaceCard>
  );
}

export function TaskDetailsPage() {
  const task = useMemo(() => getTaskDetail(), []);
  const [status, setStatus] = useState(task.status);
  const [subtasks, setSubtasks] = useState(task.subtasks);
  const [notice, setNotice] = useState("Task execution context ready.");
  const completed = subtasks.filter((item) => item.done).length;

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search task context...">
      <PageHeader
        breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "Core Infrastructure Update" }, { label: task.id }]}
        title="Task Details"
        actions={<Button onClick={() => {
          setStatus("In Progress");
          setNotice("Task execution queued");
        }}>Execute Task</Button>}
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <SurfaceCard>
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div>
                <div className="mb-2 flex flex-wrap items-center gap-2">
                  <span className="font-mono text-sm text-[var(--color-on-surface-variant)]">{task.id}</span>
                  <SelectInput className="h-8 w-40" value={status} onChange={(event) => setStatus(event.target.value)}>
                    {["To Do", "In Progress", "Review", "Completed"].map((item) => <option key={item}>{item}</option>)}
                  </SelectInput>
                  <Badge tone="error">{task.priority}</Badge>
                </div>
                <h2 className="text-2xl font-semibold">{task.title}</h2>
              </div>
              <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] px-3 py-2 text-sm">
                <span className="block text-[11px] uppercase text-[var(--color-on-surface-variant)]">Assigned Agent</span>
                <span className="font-medium">{task.agent}</span>
              </div>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <h3 className="border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Description & Requirements</h3>
            <p className="mt-4 text-sm leading-6">{task.description}</p>
            <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-[var(--color-on-surface-variant)]">
              {task.requirements.map((item) => <li key={item}>{item}</li>)}
            </ul>
          </SurfaceCard>
          <div className="grid gap-5 md:grid-cols-2">
            <SurfaceCard><h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Resource Inheritance</h3><div className="rounded-lg bg-[var(--color-surface-container-low)] p-3">Model Config → gpt-4-turbo-config-v2</div></SurfaceCard>
            <SurfaceCard><h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Linked Resources</h3>{task.resources.map((item) => <button className="mb-2 flex w-full rounded-lg border border-transparent p-2 text-left hover:border-[var(--color-outline-variant)] hover:bg-[var(--color-surface-container-low)]" type="button" key={item} onClick={() => setNotice(`${item} resource selected`)}>{item}</button>)}</SurfaceCard>
          </div>
          <SurfaceCard>
            <div className="mb-3 flex items-center justify-between border-b border-[var(--color-outline-variant)] pb-2">
              <h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Execution Timeline</h3>
              <Badge tone="accent">Live</Badge>
            </div>
            <div className="space-y-2 rounded-lg bg-[var(--color-surface-container-low)] p-4 font-mono text-[13px]">
              {task.timeline.map((event) => <div className="flex gap-4" key={event.time}><span className="text-[var(--color-on-surface-variant)]">{event.time}</span><span>{event.message}</span></div>)}
            </div>
          </SurfaceCard>
        </div>
        <aside className="space-y-5">
          <SurfaceCard>
            <h3 className="border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Task Metadata</h3>
            <div className="mt-4 space-y-3 text-sm">
              {Object.entries(task.metadata).map(([key, value]) => <div className="flex justify-between gap-3" key={key}><span className="text-[var(--color-on-surface-variant)]">{key}</span><span className="font-medium">{value}</span></div>)}
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <div className="mb-3 flex items-center justify-between border-b border-[var(--color-outline-variant)] pb-2"><h3 className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Sub-tasks</h3><span className="text-xs">{completed}/{subtasks.length} Completed</span></div>
            <div className="space-y-2">
              {subtasks.map((item) => (
                <label className="block rounded-lg p-2 hover:bg-[var(--color-surface-container-low)]" key={item.id}>
                  <div className="flex items-start gap-3">
                    <input type="checkbox" checked={item.done} onChange={() => setSubtasks((rows) => rows.map((row) => row.id === item.id ? { ...row, done: !row.done, status: !row.done ? "Completed" : "Pending", progress: !row.done ? 100 : row.progress } : row))} />
                    <div className="flex-1">
                      <div className={cn("text-sm", item.done ? "text-[var(--color-on-surface-variant)] line-through" : "")}>{item.title}</div>
                      <Badge tone={badgeToneByStatus(item.status)}>{item.status}</Badge>
                      <div className="mt-2"><ProgressBar value={item.progress} /></div>
                    </div>
                  </div>
                </label>
              ))}
            </div>
            <Button className="mt-3 w-full" variant="secondary" onClick={() => {
              const nextIndex = subtasks.length + 1;
              setSubtasks((rows) => [...rows, { id: `sub-${nextIndex}`, title: `Draft follow-up ${nextIndex}`, done: false, status: "Pending", progress: 0 }]);
              setNotice(`Sub-task ${nextIndex} added`);
            }}><Plus size={14} aria-hidden="true" />Add Sub-task</Button>
          </SurfaceCard>
        </aside>
      </div>
    </TronShell>
  );
}

export function TasksKanbanPage() {
  const columns = useMemo(() => listKanbanColumns(), []);
  const [query, setQuery] = useState("");
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);
  const [notice, setNotice] = useState("Kanban board ready.");
  const visibleColumns = columns.map((column) => ({ ...column, tasks: column.tasks.filter((task) => task.title.toLowerCase().includes(query.toLowerCase()) || task.id.toLowerCase().includes(query.toLowerCase())) }));

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search tasks...">
      <PageHeader
        title="Task Management"
        status={selectedTask ? selectedTask.id : "Kanban"}
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice("Kanban sync completed")}><RefreshCcw size={16} aria-hidden="true" />Sync</Button>
            <Button onClick={() => setNotice("Create task workflow opened")}><Plus size={16} aria-hidden="true" />Create Task</Button>
          </>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <SurfaceCard className="mb-4 p-4">
        <SearchField placeholder="Filter kanban cards..." value={query} onChange={setQuery} />
      </SurfaceCard>
      <div className="flex h-[720px] gap-4 overflow-x-auto pb-4">
        {visibleColumns.map((column) => (
          <section className="flex w-[320px] shrink-0 flex-col gap-3 rounded-lg bg-[var(--color-surface-container)] p-3" key={column.id}>
            <div className="flex items-center justify-between border-b-2 px-1 py-1" style={{ borderColor: column.accent }}>
              <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{column.title}</span>
              <span className="rounded bg-[var(--color-surface-container-high)] px-2 text-[11px]">{column.tasks.length}</span>
            </div>
            <div className="space-y-3 overflow-y-auto">
              {column.tasks.map((task) => <KanbanCard key={task.id} task={task} selected={selectedTask?.id === task.id} onSelect={() => setSelectedTask(task)} />)}
            </div>
          </section>
        ))}
      </div>
    </TronShell>
  );
}

function KanbanCard({ task, selected, onSelect }: { task: KanbanTask; selected: boolean; onSelect: () => void }) {
  return (
    <button className={cn("w-full rounded-lg border bg-white p-3 text-left shadow-sm hover:border-[var(--color-outline)]", selected ? "border-[var(--color-primary)]" : "border-[var(--color-outline-variant)]")} type="button" onClick={onSelect}>
      <div className="mb-2 flex items-center justify-between">
        <span className="font-mono text-[11px] text-[var(--color-on-surface-variant)]">{task.id}</span>
        <Badge tone={task.priority === "HIGH" ? "error" : task.priority === "MED" ? "warning" : "neutral"}>{task.priority}</Badge>
      </div>
      <h3 className="mb-3 text-sm font-medium leading-5">{task.title}</h3>
      <ProgressBar value={task.progress} />
      <div className="mt-3 flex items-center justify-between">
        <span className="flex size-6 items-center justify-center rounded-full bg-[var(--color-surface-container-high)] text-[10px] font-semibold">{task.owner}</span>
        <span className="text-[11px] text-[var(--color-on-surface-variant)]">{task.signal}</span>
      </div>
    </button>
  );
}

export function UsersPage() {
  const [users, setUsers] = useState(() => listUsers());
  const [tab, setTab] = useState("users");
  const [query, setQuery] = useState("");
  const [role, setRole] = useState("All");
  const [notice, setNotice] = useState("User directory ready.");
  const filtered = users.filter((user) => (role === "All" || user.role === role) && `${user.name} ${user.email}`.toLowerCase().includes(query.toLowerCase()));

  function updateUserRole(id: string, value: UserRecord["role"]) {
    setUsers((rows) => rows.map((user) => (user.id === id ? { ...user, role: value } : user)));
    setNotice(`Role updated to ${value}`);
  }

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users or roles...">
      <PageHeader title="Users & Roles" actions={<><Button variant="secondary" onClick={() => setNotice(`${filtered.length} users exported`)}>Export</Button><Button onClick={() => setNotice("Add user drawer opened")}><UserPlus size={16} aria-hidden="true" />Add User</Button></>} />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <div className="mb-5 grid gap-4 xl:grid-cols-3">
        <MetricCard label="Users" value={String(users.length)} trend="Across enterprise workspace" icon={<ShieldCheck size={18} aria-hidden="true" />} />
        <MetricCard label="Active" value={String(users.filter((u) => u.status === "Active").length)} trend="Signed in recently" />
        <MetricCard label="Roles" value="3" trend="Admin, Editor, Viewer" />
      </div>
      <SurfaceCard className="mb-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <SegmentedTabs value={tab} onValueChange={setTab} items={[{ label: "User Directory", value: "users" }, { label: "Role Matrix", value: "roles" }, { label: "Audit Logs", value: "audit" }]} />
          <div className="flex gap-2">
            <SearchField placeholder="Filter users..." value={query} onChange={setQuery} />
            <SelectInput className="h-9" value={role} onChange={(event) => setRole(event.target.value)}>
              {["All", "Admin", "Editor", "Viewer"].map((item) => <option key={item}>{item}</option>)}
            </SelectInput>
          </div>
        </div>
      </SurfaceCard>
      <DataTable<UserRecord>
        title={tab === "users" ? "User Directory" : tab === "roles" ? "Role Matrix Preview" : "Audit Logs Preview"}
        rows={filtered}
        rowKey={(row) => row.id}
        pagination={<span>{filtered.length} users visible · page 1 of 3</span>}
        columns={[
          { id: "name", header: "User", render: (row) => <div><div className="font-medium">{row.name}</div><div className="text-[13px] text-[var(--color-on-surface-variant)]">{row.email}</div></div> },
          { id: "role", header: "Role", render: (row) => <SelectInput className="h-8" value={row.role} onChange={(event) => updateUserRole(row.id, event.target.value as UserRecord["role"])}><option>Admin</option><option>Editor</option><option>Viewer</option></SelectInput> },
          { id: "workspace", header: "Workspace", render: (row) => row.workspace },
          { id: "status", header: "Status", render: (row) => <Badge tone={badgeToneByStatus(row.status)}>{row.status}</Badge> },
          { id: "last", header: "Last Active", render: (row) => row.lastActive },
          { id: "menu", header: "", align: "right", render: (row) => <Button size="icon" variant="ghost" aria-label={`Open actions for ${row.name}`} onClick={() => setNotice(`${row.name} action menu opened`)}><MoreHorizontal size={16} aria-hidden="true" /></Button> },
        ]}
      />
    </TronShell>
  );
}

function VariablesBasePage({ variant }: { variant: "standard" | "details" }) {
  const variables = useMemo(() => listVariables(), []);
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(variables[0]?.id ?? "");
  const [revealed, setRevealed] = useState(false);
  const [notice, setNotice] = useState("Variable inspector ready.");
  const selected = variables.find((variable) => variable.id === selectedId) ?? variables[0];
  const filtered = variables.filter((variable) => variable.key.toLowerCase().includes(query.toLowerCase()));

  return (
    <TronShell activeItem="Variables" searchPlaceholder="Search variables...">
      {variant === "details" ? <PageHeader title="Variable Management" status="Details" actions={<Button onClick={() => setNotice("New variable editor opened")}><Plus size={16} aria-hidden="true" />New Variable</Button>} /> : null}
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <div className="grid min-h-[760px] gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <SurfaceCard className="p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
            <SearchField placeholder="Search variables..." value={query} onChange={setQuery} />
            {variant === "standard" ? <Button onClick={() => setNotice("New variable editor opened")}><Plus size={16} aria-hidden="true" />New Variable</Button> : null}
          </div>
          <DataTable<VariableRecord>
            rows={filtered}
            rowKey={(row) => row.id}
            pagination={<span>Page 1 of 3 · {filtered.length} variables visible</span>}
            columns={[
              { id: "key", header: "Key", render: (row) => <button className="font-mono text-[13px] font-medium" type="button" onClick={() => setSelectedId(row.id)}>{row.key}</button> },
              { id: "scope", header: "Scope", render: (row) => <Badge>{row.scope}</Badge> },
              { id: "type", header: "Type", render: (row) => row.type },
              { id: "status", header: "Status", render: (row) => <Badge tone={badgeToneByStatus(row.status)}>{row.status}</Badge> },
              { id: "updated", header: "Updated", render: (row) => row.updated },
              { id: "actions", header: "", align: "right", render: (row) => <div className="flex justify-end gap-1"><Button size="icon" variant="ghost" aria-label={`Edit ${row.key}`} onClick={() => {
                setSelectedId(row.id);
                setNotice(`${row.key} selected for editing`);
              }}><KeyRound size={15} aria-hidden="true" /></Button><Button size="icon" variant="ghost" aria-label={`Delete ${row.key}`} onClick={() => {
                setSelectedId(row.id);
                setNotice(`Delete ${row.key} requested`);
              }}><X size={15} aria-hidden="true" /></Button></div> },
            ]}
          />
        </SurfaceCard>
        <SurfaceCard>
          <h2 className="text-xl font-semibold">{selected.key}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{selected.scope} · {selected.type}</p>
          <div className="mt-5 rounded-lg bg-[var(--color-surface-container-low)] p-4">
            <div className="flex items-center justify-between">
              <span className="font-mono text-sm">{revealed ? selected.valuePreview.replace("...", "_live_secret_") : selected.valuePreview}</span>
              <Button size="sm" variant="ghost" onClick={() => setRevealed((value) => !value)}>
                {revealed ? <EyeOff size={14} aria-hidden="true" /> : <Eye size={14} aria-hidden="true" />}
                {revealed ? "Hide" : "Reveal"}
              </Button>
            </div>
          </div>
          <h3 className="mt-8 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Usage in Prompts</h3>
          <div className="mt-3 space-y-2">
            {selected.usage.map((usage) => <button className="flex w-full items-center justify-between rounded-lg border border-[var(--color-outline-variant)] p-3 text-left text-sm hover:bg-[var(--color-surface-container-low)]" type="button" key={usage} onClick={() => setNotice(`${usage} usage link opened`)}><span>{usage}</span><ChevronRight size={15} aria-hidden="true" /></button>)}
          </div>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}

export function VariablesPage() {
  return <VariablesBasePage variant="standard" />;
}

export function VariableDetailsPage() {
  return <VariablesBasePage variant="details" />;
}

export const prototypePagesC: Record<string, PrototypeComponent> = {
  observability_dashboard_light: ObservabilityPage,
  prompt_management_tron_standardized: PromptsPage,
  resources_management_tron: ResourcesPage,
  role_matrix_tron_standardized: RoleMatrixPage,
  skills_mcp_management_tron: SkillsPage,
  task_details_lumina_workspace_updated_hierarchy: TaskDetailsPage,
  task_management_light: TasksKanbanPage,
  user_permissions_tron: UsersPage,
  variable_management_tron_standardized_1: VariablesPage,
  variable_management_tron_standardized_2: VariableDetailsPage,
};
