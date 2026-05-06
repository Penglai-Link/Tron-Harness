import { useMemo, useState, type ReactNode } from "react";
import {
  AlertCircle,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Copy,
  Database,
  Eye,
  EyeOff,
  FileArchive,
  Filter,
  FolderOpen,
  Gauge,
  Image,
  KeyRound,
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
import { ClientLink, useNavigate } from "@/lib/client-router";
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
} from "@/services/console/governance-data";
import { cn } from "@/lib/utils";

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
