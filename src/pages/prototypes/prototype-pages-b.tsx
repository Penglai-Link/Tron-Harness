import { useMemo, useState } from "react";
import {
  ArrowLeft,
  Bot,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  Code2,
  Database,
  Download,
  Eye,
  FolderOpen,
  GitBranch,
  Maximize2,
  MessageSquare,
  Minus,
  Network,
  Plus,
  RefreshCw,
  Search,
  Send,
  Settings2,
  Shield,
  Sparkles,
  Terminal,
  Upload,
  X,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable, Field, MetricCard, PageHeader, SegmentedTabs, SelectInput, SurfaceCard, TextArea, TextInput, TronShell, useWorkspaceSpaces } from "@/components/tron";
import { ClientLink, useNavigate } from "@/lib/client-router";
import { cn } from "@/lib/utils";
import {
  getDashboardData,
  getGraphData,
  getImportApplicationDefaults,
  getMainAgentMessages,
  getMemoryFragments,
  getModels,
  type GraphEntity,
  type ImportApplicationDefaults,
  type MemoryFragment,
  type ModelRegistryItem,
} from "@/services/prototypes/prototype-data-b";

function PrototypeShell({ activeItem, children }: { activeItem: string; children: React.ReactNode }) {
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
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

export function DashboardPage() {
  return (
    <PrototypeShell activeItem="Dashboard">
      <DashboardContent />
    </PrototypeShell>
  );
}

function DashboardContent() {
  const { activities, metrics } = useMemo(getDashboardData, []);
  const { secondarySpaces, selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const [expanded, setExpanded] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const primaryName = selectedPrimarySpace?.name ?? "Enterprise Workspace";
  const primaryMeta = selectedPrimarySpace
    ? `${selectedPrimarySpace.region} · ${selectedPrimarySpace.status} · ${selectedPrimarySpace.description}`
    : "Global · Active · Global governance, security, and shared AI resource policy.";

  return (
    <>
      <PageHeader
        title="Workspace Overview"
        description="Monitor agent health, task execution, and system inheritance."
        actions={
          <Button variant="secondary" onClick={() => setShowAll((current) => !current)}>
            <Download size={16} aria-hidden="true" />
            Export Report
          </Button>
        }
      />
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {metrics.map((metric) => (
          <MetricCard
            key={metric.label}
            label={metric.label}
            value={metric.value}
            trend={metric.trend}
            icon={getDashboardMetricIcon(metric.label)}
          />
        ))}
      </div>

      <div className="mt-5 grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(320px,1fr)]">
        <SurfaceCard>
          <div className="mb-4 flex items-center justify-between border-b border-[var(--color-outline-variant)] pb-3">
            <h2 className="text-xl font-semibold">Inheritance Map</h2>
            <Button size="icon" variant="ghost" aria-pressed={expanded} onClick={() => setExpanded((current) => !current)}>
              <Maximize2 size={17} aria-hidden="true" />
            </Button>
          </div>
          <div className={cn("space-y-4 text-sm", expanded ? "min-h-[360px]" : "min-h-[260px]")}>
            <TreeNode title={primaryName} meta={primaryMeta} root />
            <div className="ml-4 border-l border-[var(--color-outline-variant)] pl-8">
              {secondarySpaces.length ? (
                secondarySpaces.map((space) => {
                  const active = space.id === selectedSecondarySpace?.id;
                  return (
                    <TreeNode
                      active={active}
                      key={space.id}
                      title={`${space.name}${active ? " (Current)" : ""}`}
                      meta={`${space.description} · ${space.inheritedResources} inherited resources · ${space.localOverrides} local overrides · ${space.status}`}
                    />
                  );
                })
              ) : (
                <p className="rounded-lg bg-[var(--color-surface-container-low)] p-3 text-[13px] text-[var(--color-on-surface-variant)]">No secondary spaces configured for this primary space.</p>
              )}
            </div>
          </div>
          <p className="mt-3 text-[13px] text-[var(--color-on-surface-variant)]">{expanded ? "Expanded map controls are visible." : "Current navigation context."}</p>
        </SurfaceCard>

        <SurfaceCard className="p-0">
          <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-5 py-4">
            <h2 className="text-xl font-semibold">Recent Activity</h2>
            <button className="text-[11px] font-semibold uppercase text-[var(--color-accent)]" type="button" onClick={() => setShowAll((current) => !current)}>
              View All
            </button>
          </div>
          <ul className="max-h-[360px] overflow-y-auto p-3">
            {activities.slice(0, showAll ? activities.length : 4).map((activity) => (
              <li className="flex gap-3 rounded-lg p-3 hover:bg-[var(--color-hover-surface)]" key={activity.title}>
                <span className="mt-0.5 flex size-7 items-center justify-center rounded-full bg-[var(--color-surface-container)]">
                  {activity.tone === "warning" ? <CircleAlert size={15} /> : <CheckCircle2 size={15} />}
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-sm">{activity.title}</span>
                  <span className="block text-[12px] text-[var(--color-on-surface-variant)]">{activity.meta}</span>
                </span>
              </li>
            ))}
          </ul>
        </SurfaceCard>
      </div>
    </>
  );
}

function getDashboardMetricIcon(label: string) {
  switch (label) {
    case "System Health":
      return <Shield size={18} aria-hidden="true" />;
    case "Active Agents":
      return <Bot size={18} aria-hidden="true" />;
    case "Running Tasks":
      return <RefreshCw size={18} aria-hidden="true" />;
    case "Models":
      return <Sparkles size={18} aria-hidden="true" />;
    case "MCP Servers":
      return <Network size={18} aria-hidden="true" />;
    case "Skills":
      return <Zap size={18} aria-hidden="true" />;
    case "Apps":
      return <FolderOpen size={18} aria-hidden="true" />;
    case "Memory Items":
      return <Database size={18} aria-hidden="true" />;
    default:
      return <Settings2 size={18} aria-hidden="true" />;
  }
}

function TreeNode({ active, meta, root, title }: { active?: boolean; meta: string; root?: boolean; title: string }) {
  return (
    <div className={cn("flex items-start gap-3 rounded-lg p-3", active ? "border border-[var(--color-primary)] bg-white" : root ? "" : "bg-[var(--color-surface-container-low)]")}>
      <span className={cn("flex size-9 shrink-0 items-center justify-center rounded-full", active ? "bg-[var(--color-primary)] text-white" : "bg-[var(--color-surface-container-high)]")}>
        <Network size={16} aria-hidden="true" />
      </span>
      <span>
        <span className="block font-semibold">{title}</span>
        <span className="mt-1 block text-[13px] text-[var(--color-on-surface-variant)]">{meta}</span>
      </span>
    </div>
  );
}

function ImportApplicationPage({ initialSource }: { initialSource: ImportApplicationDefaults["source"] }) {
  const defaults = useMemo(() => getImportApplicationDefaults(initialSource), [initialSource]);
  const sourceOptions: Array<{ value: ImportApplicationDefaults["source"]; label: string; icon: LucideIcon }> = [
    { value: "github", label: "GitHub", icon: Code2 },
    { value: "gitlab", label: "GitLab", icon: Terminal },
    { value: "local", label: "Local Directory", icon: FolderOpen },
  ];
  const [source, setSource] = useState(defaults.source);
  const [form, setForm] = useState(defaults);
  const [tags, setTags] = useState(defaults.tags);
  const [feedback, setFeedback] = useState("Ready to import");

  function update(field: keyof ImportApplicationDefaults, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
  }

  return (
    <PrototypeShell activeItem="Apps">
      <div className="mx-auto max-w-3xl">
        {initialSource === "local" ? (
          <ClientLink className="mb-6 inline-flex items-center gap-1 text-sm text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]" href="/apps">
            <ArrowLeft size={16} aria-hidden="true" />
            Back to Apps
          </ClientLink>
        ) : null}
        <SurfaceCard className="overflow-hidden p-0">
          <div className="border-b border-[var(--color-outline-variant)] px-8 py-6">
            <h1 className="text-2xl font-semibold">Import Application</h1>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">Connect a repository or upload local files to deploy a new app.</p>
          </div>
          <div className="space-y-6 p-8">
            <div>
              <p className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Source</p>
              <div className="grid gap-3 sm:grid-cols-3">
                {sourceOptions.map(({ icon: Icon, label, value }) => (
                  <button
                    className={cn(
                      "flex flex-col items-center justify-center rounded-lg border p-4 text-sm transition-colors",
                      source === value ? "border-2 border-[var(--color-primary)] text-[var(--color-primary)]" : "border-[var(--color-outline-variant)] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]",
                    )}
                    key={value}
                    type="button"
                    onClick={() => {
                      setSource(value);
                      update("source", value);
                    }}
                  >
                    <Icon size={24} aria-hidden="true" />
                    <span className="mt-2">{label}</span>
                  </button>
                ))}
              </div>
            </div>

            {source !== "local" ? (
              <>
                <Field label="Repository URL">
                  <TextInput value={form.repositoryUrl} placeholder="https://github.com/username/repo" onChange={(event) => update("repositoryUrl", event.target.value)} />
                </Field>
                <Field label="Access Token (Private Repos)" hint="Optional">
                  <TextInput type="password" value={form.accessToken} placeholder="ghp_xxxxxxxxxxxxxxxxxxxx" onChange={(event) => update("accessToken", event.target.value)} />
                </Field>
              </>
            ) : null}
            <Field label="Local File System Path" hint={source !== "local" ? "Directory where the repository will be cloned." : undefined}>
              <TextInput value={form.localPath} placeholder="/Users/username/projects/my-app" onChange={(event) => update("localPath", event.target.value)} />
            </Field>
            <Field label="App Name">
              <TextInput value={form.appName} placeholder="my-awesome-app" onChange={(event) => update("appName", event.target.value)} />
            </Field>
            <Field label="Tags">
              <div className="flex min-h-10 flex-wrap items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-white p-2">
                {tags.map((tag) => (
                  <span className="inline-flex items-center gap-1 rounded-full bg-[var(--color-accent-container)] px-2.5 py-1 text-[13px] text-[var(--color-on-accent-container)]" key={tag}>
                    {tag}
                    <button type="button" onClick={() => setTags((current) => current.filter((item) => item !== tag))}>
                      <X size={13} aria-hidden="true" />
                    </button>
                  </span>
                ))}
                <input className="min-w-[120px] flex-1 bg-transparent text-sm outline-none" placeholder="Add tags..." onKeyDown={(event) => {
                  if (event.key === "Enter" && event.currentTarget.value.trim()) {
                    setTags((current) => [...current, event.currentTarget.value.trim()]);
                    event.currentTarget.value = "";
                  }
                }} />
              </div>
            </Field>
            {source !== "local" ? (
              <Field label="Branch / Ref">
                <TextInput value={form.branch} onChange={(event) => update("branch", event.target.value)} />
              </Field>
            ) : null}
          </div>
          <div className="flex items-center justify-between border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-8 py-5">
            <span className="text-[13px] text-[var(--color-on-surface-variant)]">{feedback}</span>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={() => setFeedback("Import canceled")}>Cancel</Button>
              <Button onClick={() => setFeedback(`Import queued for ${form.appName || "new app"}`)}>
                <Upload size={16} aria-hidden="true" />
                Import App
              </Button>
            </div>
          </div>
        </SurfaceCard>
      </div>
    </PrototypeShell>
  );
}

export function ImportApplicationGithubPage() {
  return <ImportApplicationPage initialSource="github" />;
}

export function ImportApplicationLocalPage() {
  return <ImportApplicationPage initialSource="local" />;
}

function GraphPage({ domainFiltering = false }: { domainFiltering?: boolean }) {
  const [domain, setDomain] = useState("customer-support");
  const [query, setQuery] = useState("");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [zoom, setZoom] = useState(100);
  const graph = useMemo(() => getGraphData(domain), [domain]);
  const visibleEntities = graph.entities.filter((entity) => entity.label.toLowerCase().includes(query.toLowerCase()) || entity.type.toLowerCase().includes(query.toLowerCase()));
  const [selectedId, setSelectedId] = useState("customer-0192");
  const selected = graph.entities.find((entity) => entity.id === selectedId) ?? graph.entities[0];

  return (
    <PrototypeShell activeItem="Graph">
      <div className="grid min-h-[720px] overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white xl:grid-cols-[300px_minmax(520px,1fr)_320px]">
        <aside className="border-r border-[var(--color-outline-variant)] bg-white">
          <div className="border-b border-[var(--color-outline-variant)] p-4">
            <h1 className="text-xl font-semibold">Entities</h1>
            {domainFiltering ? (
              <Field label="Domain">
                <SelectInput value={domain} onChange={(event) => setDomain(event.target.value)}>
                  <option value="customer-support">Domain: Customer Support</option>
                  <option value="supply-chain">Domain: Supply Chain</option>
                  <option value="hr">Domain: HR</option>
                </SelectInput>
              </Field>
            ) : null}
            <div className="mt-3 flex h-10 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] px-3">
              <Search size={16} aria-hidden="true" />
              <input className="min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder={domainFiltering ? `Search ${graph.domainLabel}...` : "Filter nodes..."} value={query} onChange={(event) => setQuery(event.target.value)} />
            </div>
          </div>
          <div className="space-y-1 p-3">
            {visibleEntities.map((entity) => (
              <button
                className={cn("flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-[var(--color-hover-surface)]", selected.id === entity.id ? "bg-[var(--color-surface-container-high)]" : "")}
                key={entity.id}
                type="button"
                onClick={() => {
                  setSelectedId(entity.id);
                  setInspectorOpen(true);
                }}
              >
                <span className="flex size-8 items-center justify-center rounded-full bg-[var(--color-surface-container)]">
                  <Database size={15} aria-hidden="true" />
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium">{entity.label}</span>
                  <span className="block truncate text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{entity.type}</span>
                </span>
              </button>
            ))}
          </div>
        </aside>
        <GraphCanvas entities={graph.entities} selected={selected} setSelectedId={setSelectedId} setInspectorOpen={setInspectorOpen} setZoom={setZoom} zoom={zoom} />
        {inspectorOpen ? <GraphInspector selected={selected} onClose={() => setInspectorOpen(false)} /> : null}
      </div>
    </PrototypeShell>
  );
}

function GraphCanvas({ entities, selected, setInspectorOpen, setSelectedId, setZoom, zoom }: { entities: GraphEntity[]; selected: GraphEntity; setInspectorOpen: (open: boolean) => void; setSelectedId: (id: string) => void; setZoom: React.Dispatch<React.SetStateAction<number>>; zoom: number }) {
  return (
    <section className="relative min-h-[720px] overflow-hidden bg-[var(--color-surface-container-low)]">
      <div className="absolute right-4 top-4 z-10 flex overflow-hidden rounded-lg border border-[var(--color-outline-variant)] bg-white">
        <Button size="icon" variant="ghost" onClick={() => setZoom((current) => current + 10)}><Plus size={16} /></Button>
        <Button size="icon" variant="ghost" onClick={() => setZoom((current) => Math.max(60, current - 10))}><Minus size={16} /></Button>
        <Button size="icon" variant="ghost" onClick={() => setZoom(100)}><Maximize2 size={16} /></Button>
      </div>
      <span className="absolute left-4 top-4 rounded-full bg-white px-3 py-1 text-[12px] text-[var(--color-on-surface-variant)]">Zoom {zoom}%</span>
      <svg className="absolute inset-0 h-full w-full" aria-hidden="true">
        <path d="M 350 250 L 500 400" fill="none" stroke="#c4c7c7" strokeDasharray="4 4" strokeWidth="2" />
        <path d="M 350 250 L 200 400" fill="none" stroke="#c4c7c7" strokeWidth="2" />
        <path d="M 500 400 L 650 300" fill="none" stroke="#c4c7c7" strokeWidth="2" />
      </svg>
      {entities.map((entity) => (
        <button
          className="absolute z-10 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center"
          style={{ left: entity.x, top: entity.y, transform: `translate(-50%, -50%) scale(${zoom / 100})` }}
          key={entity.id}
          type="button"
          onClick={() => {
            setSelectedId(entity.id);
            setInspectorOpen(true);
          }}
        >
          <span className={cn("flex rounded-full shadow-sm", selected.id === entity.id ? "size-14 bg-[var(--color-primary)] text-white ring-4 ring-[var(--color-accent-container)]" : "size-12 border border-[var(--color-outline-variant)] bg-white")}>
            <Network className="m-auto" size={20} aria-hidden="true" />
          </span>
          <span className="mt-2 whitespace-nowrap rounded-lg border border-[var(--color-outline-variant)] bg-white px-2 py-1 text-sm shadow-sm">{entity.label}</span>
        </button>
      ))}
    </section>
  );
}

function GraphInspector({ onClose, selected }: { onClose: () => void; selected: GraphEntity }) {
  const [notice, setNotice] = useState("Entity properties ready.");

  return (
    <aside className="border-l border-[var(--color-outline-variant)] bg-white">
      <div className="flex items-start justify-between border-b border-[var(--color-outline-variant)] p-4">
        <div>
          <h2 className="text-xl font-semibold">{selected.label}</h2>
          <p className="text-[12px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{selected.type} Node</p>
        </div>
        <Button size="icon" variant="ghost" onClick={onClose}><X size={16} /></Button>
      </div>
      <div className="space-y-5 p-4">
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Properties</h3>
          <div className="overflow-hidden rounded-lg border border-[var(--color-outline-variant)]">
            {Object.entries(selected.properties).map(([key, value]) => (
              <div className="flex justify-between gap-3 border-b border-[var(--color-outline-variant)] px-3 py-2 text-sm last:border-0" key={key}>
                <span className="text-[var(--color-on-surface-variant)]">{key}</span>
                <span className="text-right font-medium">{value}</span>
              </div>
            ))}
          </div>
        </section>
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Relationships ({selected.relationships.length})</h3>
          <div className="space-y-2">
            {selected.relationships.map((relationship) => (
              <div className="rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={`${relationship.label}-${relationship.target}`}>
                <p className="text-[var(--color-on-surface-variant)]">{relationship.label}</p>
                <p className="font-medium">{relationship.target}</p>
              </div>
            ))}
          </div>
        </section>
        <Button className="w-full" onClick={() => setNotice(`Editing ${selected.label}`)}>Edit Entity</Button>
        <Notice value={notice} />
      </div>
    </aside>
  );
}

export function KnowledgeGraphPage() {
  return <GraphPage />;
}

export function KnowledgeGraphDomainPage() {
  return <GraphPage domainFiltering />;
}

export function LoginPage() {
  const { t } = useTranslation(["auth", "common"]);
  const [captcha, setCaptcha] = useState("K7Q4");
  const [status, setStatus] = useState(() => t("auth:status"));
  const navigate = useNavigate();

  return (
    <main className="flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6 py-10 text-[var(--color-on-surface)]">
      <section className="w-full max-w-md rounded-xl border border-[var(--color-outline-variant)] bg-white p-10 shadow-sm">
        <div className="mb-8 text-center">
          <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-xl bg-[var(--color-primary)] text-white">
            <Sparkles size={22} aria-hidden="true" />
          </div>
          <h1 className="text-2xl font-semibold">{t("auth:title")}</h1>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{t("auth:subtitle")}</p>
        </div>
        <form className="space-y-5" onSubmit={(event) => {
          event.preventDefault();
          setStatus(t("auth:signingIn"));
          navigate("/dashboard");
        }}>
          <Field label={t("auth:fields.operatorId")}>
            <TextInput placeholder="TRON-XXXX" />
          </Field>
          <Field label={t("auth:fields.passkey")}>
            <TextInput type="password" placeholder={t("auth:passkeyPlaceholder")} />
          </Field>
          <div>
            <div className="mb-2 flex items-center justify-between">
              <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{t("auth:captcha.label")}</span>
              <Button size="sm" variant="ghost" type="button" onClick={() => setCaptcha((current) => (current === "K7Q4" ? "M2R8" : "K7Q4"))}>
                <RefreshCw size={14} aria-hidden="true" />
                {t("common:actions.refresh")}
              </Button>
            </div>
            <div className="flex gap-3">
              <div className="flex w-1/3 items-center justify-center rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] font-mono text-lg font-semibold tracking-[0.2em]">{captcha}</div>
              <TextInput className="w-2/3" placeholder={t("auth:captcha.placeholder")} />
            </div>
          </div>
          <Button className="w-full" type="submit">{t("common:actions.signIn")}</Button>
          <div className="flex justify-between text-[13px] text-[var(--color-on-surface-variant)]">
            <button type="button" onClick={() => setStatus(t("auth:recoveryRequested"))}>{t("auth:forgotPasskey")}</button>
            <span>{status}</span>
          </div>
        </form>
        <p className="mt-10 text-center text-xs text-[var(--color-on-surface-variant)]">{t("auth:copyright")}</p>
      </section>
    </main>
  );
}

export function MainAgentPage() {
  const messages = useMemo(getMainAgentMessages, []);
  const [open, setOpen] = useState(true);
  const [minimized, setMinimized] = useState(false);
  const [draft, setDraft] = useState("");
  const [status, setStatus] = useState("Executing... 0.8s");

  return (
    <PrototypeShell activeItem="Dashboard">
      <div className="relative min-h-[760px]">
        <div className="grid gap-4 opacity-55 lg:grid-cols-3">
          <MetricCard label="System Health" value="99.98%" trend="All services operational" />
          <MetricCard label="Active Agents" value="42" trend="+3 since last hour" />
          <MetricCard label="Pending Tasks" value="1,204" trend="Avg processing time: 1.2s" />
          <SurfaceCard className="lg:col-span-2"><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Throughput Analytics</h2><div className="h-48 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)]" /></SurfaceCard>
          <SurfaceCard><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Recent Activity</h2><p className="text-sm">Deployment triggered in us-east-1</p></SurfaceCard>
        </div>
        <div className="absolute inset-0 bg-[rgba(253,248,248,0.45)]" />
        {open ? (
          <aside className={cn("fixed bottom-6 right-6 z-50 flex w-[440px] max-w-[calc(100vw-3rem)] flex-col overflow-hidden rounded-[20px] border border-[var(--color-outline-variant)] bg-white shadow-[0_8px_32px_rgba(0,0,0,0.12)]", minimized ? "h-[72px]" : "h-[680px]")}>
            <div className="flex h-[60px] items-center justify-between border-b border-[var(--color-outline-variant)] px-5">
              <div className="flex items-center gap-3">
                <span className="relative flex size-8 items-center justify-center rounded-full bg-[var(--color-primary)] text-white"><Bot size={18} /><i className="absolute bottom-0 right-0 size-2.5 rounded-full border-2 border-white bg-green-500" /></span>
                <span><span className="block text-[15px] font-semibold">Main Agent</span><span className="block text-[11px] text-[var(--color-on-surface-variant)]">Workspace Coordinator</span></span>
              </div>
              <div className="flex gap-1">
                <Button size="icon" variant="ghost" onClick={() => setMinimized((current) => !current)}><Minus size={16} /></Button>
                <Button size="icon" variant="ghost" onClick={() => setOpen(false)}><X size={16} /></Button>
              </div>
            </div>
            {!minimized ? (
              <>
                <div className="flex-1 space-y-5 overflow-y-auto bg-[#fcfafa] p-5">
                  <div className="text-center"><Badge>Today, 14:02</Badge></div>
                  {messages.map((message) => <div className={cn("rounded-2xl px-4 py-3 text-sm", message.role === "user" ? "ml-auto max-w-[85%] bg-[var(--color-surface-container)]" : "max-w-[95%] border border-[var(--color-outline-variant)] bg-white")} key={message.id}>{message.body}</div>)}
                  <SurfaceCard className="p-3">
                    <h3 className="mb-2 border-b border-[var(--color-outline-variant)] pb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Reasoning Process</h3>
                    <ol className="list-inside list-decimal space-y-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                      <li>Fetch metrics for primary clusters.</li>
                      <li>Evaluate CPU and Memory over last 15m.</li>
                      <li>Calculate standard node scale-up.</li>
                    </ol>
                  </SurfaceCard>
                  <div className="overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white">
                    <div className="flex items-center justify-between bg-[var(--color-surface-container)] px-3 py-2 text-sm"><span className="flex items-center gap-2"><Terminal size={14} />FetchClusterMetrics</span><Badge>{status}</Badge></div>
                    <pre className="overflow-x-auto bg-[#1e1e1e] p-3 text-[11px] text-gray-300">{"> GET /api/v2/metrics/clusters?tags=primary\n{\n  \"us-east-1-alpha\": { \"cpu\": \"84%\", \"mem\": \"72%\" },\n  \"eu-west-1-beta\":  { \"cpu\": \"65%\", \"mem\": \"50%\" }\n}"}</pre>
                  </div>
                  <div className="flex gap-2">
                    <Button size="sm" onClick={() => setStatus("Scale-up approved")}>Execute Scale-up</Button>
                    <Button size="sm" variant="secondary" onClick={() => setStatus("Plan review opened")}>Review Plan</Button>
                  </div>
                </div>
                <div className="border-t border-[var(--color-outline-variant)] p-4">
                  <div className="flex items-end gap-2 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-1.5">
                    <Button size="icon" variant="ghost" aria-label="Open attachment picker" onClick={() => setStatus("Attachment picker opened")}><Plus size={18} /></Button>
                    <TextArea className="min-h-9 flex-1 border-0 bg-transparent" rows={1} placeholder="Message Main Agent..." value={draft} onChange={(event) => setDraft(event.target.value)} />
                    <Button size="icon" onClick={() => setDraft("")}><Send size={16} /></Button>
                  </div>
                </div>
              </>
            ) : null}
          </aside>
        ) : (
          <Button className="fixed bottom-6 right-6 z-50" onClick={() => setOpen(true)}><Bot size={16} />Open Main Agent</Button>
        )}
      </div>
    </PrototypeShell>
  );
}

function MemoryRepository({ details = false }: { details?: boolean }) {
  const fragments = useMemo(getMemoryFragments, []);
  const [strategy, setStrategy] = useState("hybrid");
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState(["Namespace: UserPrefs", "Entity: usr_982x", "Confidence: > 0.85"]);
  const [selectedId, setSelectedId] = useState("mem-1");
  const [notice, setNotice] = useState("Memory search studio ready.");
  const visible = fragments.filter((fragment) => fragment.content.toLowerCase().includes(query.toLowerCase()) || fragment.namespace.toLowerCase().includes(query.toLowerCase()));
  const selected = fragments.find((fragment) => fragment.id === selectedId) ?? fragments[0];

  const table = (
    <DataTable
      title="Memory Fragments"
      rows={visible}
      rowKey={(row) => row.id}
      toolbar={<span className="text-[13px] text-[var(--color-on-surface-variant)]">Showing 1-5 of 2,491 fragments</span>}
      columns={[
        { id: "content", header: "Content Snippet", render: (row) => <button className="max-w-[380px] truncate text-left" type="button" onClick={() => setSelectedId(row.id)}>{row.content}</button> },
        { id: "namespace", header: "Namespace", render: (row) => row.namespace },
        { id: "entity", header: "Entity", render: (row) => <span className="font-mono">{row.entity}</span> },
        { id: "source", header: "Source", render: (row) => row.source },
        { id: "confidence", header: "Confidence", align: "right", render: (row) => row.confidence.toFixed(2) },
        { id: "created", header: "Created", render: (row) => row.created },
        { id: "inheritance", header: "Inheritance", render: (row) => <SmallStatus tone={row.inheritance === "Override" ? "accent" : "neutral"}>{row.inheritance}</SmallStatus> },
      ]}
      pagination={<span>{visible.length} visible fragments</span>}
    />
  );

  return (
    <PrototypeShell activeItem="Memory">
      <PageHeader
        title="Memory Repository"
        description="Manage, query, and structure agent memory fragments."
        actions={<><Button variant="secondary" onClick={() => setNotice("Import data drawer opened")}><Upload size={16} />Import Data</Button><Button onClick={() => setNotice("Index sync queued")}><RefreshCw size={16} />Sync Indices</Button></>}
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <SurfaceCard className="mb-4">
        <div className="flex flex-col gap-3 md:flex-row md:items-center">
          <div className="flex h-10 flex-1 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-white px-3">
            <Search size={16} />
            <input className="min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder="Query memory vectors via natural language or exact match..." value={query} onChange={(event) => setQuery(event.target.value)} />
          </div>
          <SegmentedTabs value={strategy} onValueChange={setStrategy} items={[{ label: "Hybrid", value: "hybrid" }, { label: "Semantic Only", value: "semantic" }, { label: "Keyword Only", value: "keyword" }]} />
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Active Filters:</span>
          {filters.map((filter) => (
            <span className="inline-flex items-center gap-1 rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-high)] px-3 py-1 text-sm" key={filter}>
              {filter}
              <button type="button" onClick={() => setFilters((current) => current.filter((item) => item !== filter))}><X size={13} /></button>
            </span>
          ))}
          <Button size="sm" variant="ghost" onClick={() => setFilters((current) => [...current, `Strategy: ${strategy}`])}><Plus size={14} />Add Filter</Button>
        </div>
      </SurfaceCard>
      {details ? <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_400px]">{table}<MemoryInspector fragment={selected} /></div> : table}
    </PrototypeShell>
  );
}

function MemoryInspector({ fragment }: { fragment: MemoryFragment }) {
  const [status, setStatus] = useState("Selected");
  return (
    <SurfaceCard className="flex flex-col p-0">
      <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] p-5">
        <h2 className="text-xl font-semibold">Memory Details</h2>
        <Button size="icon" variant="ghost" onClick={() => setStatus("Inspector closed")}><X size={16} /></Button>
      </div>
      <div className="space-y-6 p-5">
        <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Content Snippet</h3><div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm leading-6">{fragment.content} This includes removing pleasantries and preserving information density.</div></section>
        <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Metadata</h3><div className="grid grid-cols-2 gap-4 rounded-lg border border-[var(--color-outline-variant)] p-4 text-sm"><Meta label="Entity" value={fragment.entity} /><Meta label="Namespace" value={fragment.namespace} /><Meta label="Confidence" value={fragment.confidence.toFixed(2)} /><Meta label="Inheritance" value={fragment.inheritance} /><Meta label="Created" value={`${fragment.created} 14:22:45 UTC`} /></div></section>
        <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Origin</h3><div className="flex items-center gap-3 rounded-lg border border-[var(--color-outline-variant)] p-4"><Database size={18} /><span><span className="block text-sm font-medium">{fragment.source}</span><span className="block text-[12px] text-[var(--color-on-surface-variant)]">Primary Environment</span></span></div></section>
        <p className="text-[13px] text-[var(--color-on-surface-variant)]">{status}</p>
      </div>
      <div className="mt-auto flex gap-2 border-t border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
        <Button className="flex-1" variant="secondary" onClick={() => setStatus("Edit mode enabled")}>Edit Fragment</Button>
        <Button className="flex-1" onClick={() => setStatus("Fragment invalidated")}>Invalidate</Button>
      </div>
    </SurfaceCard>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return <div><p className="text-[11px] text-[var(--color-on-surface-variant)]">{label}</p><p className="mt-1 font-medium">{value}</p></div>;
}

export function MemoryPage() {
  return <MemoryRepository />;
}

export function MemoryDetailsPage() {
  return <MemoryRepository details />;
}

export function ModelRegistryPage() {
  const models = useMemo(getModels, []);
  const [vendor, setVendor] = useState("All Vendors");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState("gpt-4-turbo");
  const [notice, setNotice] = useState("Model registry ready.");
  const visible = models.filter((model) => (vendor === "All Vendors" || model.vendor === vendor) && model.name.toLowerCase().includes(query.toLowerCase()));
  const selected = models.find((model) => model.id === selectedId) ?? models[0];

  return (
    <PrototypeShell activeItem="Models">
      <div className="grid min-h-[740px] gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <section>
          <PageHeader title="Model Registry" description="Manage AI vendors, configurations, and inheritance rules." actions={<Button onClick={() => setNotice("Add model workflow opened")}><Plus size={16} />Add Model</Button>} />
          <div className="mb-4">
            <Notice value={notice} />
          </div>
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] pb-4">
            <div className="flex flex-wrap gap-2">
              {["All Vendors", "OpenAI", "Anthropic", "Google"].map((item) => (
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
    </PrototypeShell>
  );
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

export const prototypePagesB = {
  dashboard_light: DashboardPage,
  import_application_tron_unified_navigation_1: ImportApplicationGithubPage,
  import_application_tron_unified_navigation_2: ImportApplicationLocalPage,
  knowledge_graph_tron: KnowledgeGraphPage,
  knowledge_graph_tron_domain_filtering: KnowledgeGraphDomainPage,
  login_tron_light_theme_centered: LoginPage,
  main_agent_chat_tron: MainAgentPage,
  memory_management_tron_data_isolation_1: MemoryPage,
  memory_management_tron_data_isolation_2: MemoryDetailsPage,
  model_registry_tron: ModelRegistryPage,
} as const;
