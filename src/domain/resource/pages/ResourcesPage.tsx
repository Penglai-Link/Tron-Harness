import {
  useState,
  type ReactNode,
} from "react";
import {
  AlertCircle,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Database,
  FileArchive,
  Filter,
  FolderOpen,
  Image,
  MoreHorizontal,
  RefreshCcw,
  Search,
  SlidersHorizontal,
  Settings2,
  Upload,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  TextInput,
  TronShell,
} from "@/shared";

import {
  useResources,
  type ResourceRecord,
  type StatusTone,
} from "@/domain/resource/api";
import { cn } from "@/shared/lib/utils";

const badgeToneByStatus = (status: string): StatusTone => {
  const normalized = status.toLowerCase();
  if (normalized.includes("healthy") || normalized.includes("active") || normalized.includes("connected") || normalized.includes("completed")) return "success";
  if (normalized.includes("review") || normalized.includes("sync") || normalized.includes("progress") || normalized.includes("override")) return "warning";
  if (normalized.includes("suspend") || normalized.includes("disabled") || normalized.includes("deprecated")) return "error";
  return "neutral";
};

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
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

export function ResourcesPage() {
  const resources = useResources();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [typeFilter, setTypeFilter] = useState("All");
  const [statusFilter, setStatusFilter] = useState("Active");
  const [page, setPage] = useState(1);
  const [rowMenu, setRowMenu] = useState<string | null>(null);
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
    <TronShell activeItem="Resources" searchPlaceholder="Search resources...">
        <div className="flex min-h-[calc(100vh-3.5rem)] flex-col gap-6">
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
              <div className="flex flex-1 flex-wrap items-center gap-2">
                <div className="min-w-[240px] flex-1 sm:max-w-[360px]">
                  <SearchField
                    placeholder="Search resources..."
                    value={query}
                    onChange={(value) => {
                      setQuery(value);
                      setNotice(value ? `Search filtered to ${value}` : "Search cleared");
                    }}
                  />
                </div>
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
        </div>
    </TronShell>
  );
}
