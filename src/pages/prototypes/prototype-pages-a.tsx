import { useMemo, useState } from "react";
import {
  ArrowLeft,
  Bot,
  ChevronLeft,
  ChevronRight,
  CircleCheck,
  ClipboardList,
  Code2,
  Copy,
  Database,
  Eye,
  EyeOff,
  FileText,
  Filter,
  GitBranch,
  Layers3,
  Link,
  List,
  MoreHorizontal,
  Paperclip,
  Play,
  Plus,
  RefreshCw,
  Save,
  Search,
  Send,
  Settings2,
  SlidersHorizontal,
  Terminal,
  Trash2,
  Wrench,
  Zap,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable, Field, InspectorPanel, PageHeader, SegmentedTabs, SelectInput, SurfaceCard, TextArea, TextInput, ToggleSwitch, TronShell } from "@/components/tron";
import { cn } from "@/lib/utils";
import {
  getApps,
  getAuditLogs,
  getAutomations,
  getChatMessages,
  getChatSessionGroups,
  getModelCostPreview,
  getPromptVariables,
  getSkillSchemaPreview,
  getTaskQuickReference,
  type AppRecord,
  type AuditLogRecord,
  type AutomationRecord,
  type BadgeStatus,
  type ChatMessage,
} from "@/services/prototypes/prototype-data-a";

function statusTone(status: BadgeStatus | AuditLogRecord["status"]) {
  if (status === "Success" || status === "Healthy" || status === "Running") return "success";
  if (status === "Warning" || status === "Draft") return "warning";
  if (status === "Failed" || status === "Disabled") return "error";
  return "neutral";
}

function IconButton({ label, children, onClick }: { label: string; children: React.ReactNode; onClick?: () => void }) {
  return (
    <button className="inline-flex size-8 items-center justify-center rounded-lg text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]" type="button" aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function SectionTitle({ icon, title }: { icon?: React.ReactNode; title: string }) {
  return (
    <div className="mb-4 flex items-center gap-2 border-b border-[var(--color-outline-variant)] pb-3">
      {icon}
      <h2 className="text-base font-semibold">{title}</h2>
    </div>
  );
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 text-sm">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="font-mono text-[13px]">{value}</span>
    </div>
  );
}

function tagList(tags: string[]) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <Badge key={tag} tone="neutral">
          {tag}
        </Badge>
      ))}
    </div>
  );
}

const promptShellNavItems = ["Home", "Library", "Deployments", "Experiments", "Usage", "Settings"];

export function AddModelPage() {
  const costs = useMemo(() => getModelCostPreview(), []);
  const [provider, setProvider] = useState("openai");
  const [temperature, setTemperature] = useState(0.7);
  const [tokens, setTokens] = useState(8192);
  const [contextWindow, setContextWindow] = useState("128000");
  const [inheritable, setInheritable] = useState(true);
  const [showKey, setShowKey] = useState(false);
  const [notice, setNotice] = useState("Configuration preview is synced.");

  const preview = useMemo(
    () =>
      JSON.stringify(
        {
          provider,
          model: "gpt-4.1-enterprise",
          temperature,
          maxCompletionTokens: tokens,
          contextWindow,
          inheritable,
        },
        null,
        2,
      ),
    [contextWindow, inheritable, provider, temperature, tokens],
  );

  return (
    <TronShell activeItem="Models" searchPlaceholder="Search models, providers, costs...">
      <div className="-mx-6 -mt-6 mb-6 flex h-14 items-center justify-between border-b border-[var(--color-outline-variant)] bg-white px-6">
        <div className="flex items-center gap-3">
          <IconButton label="Back to models" onClick={() => setNotice("Back to Models requested")}>
            <ArrowLeft size={18} />
          </IconButton>
          <h1 className="text-base font-semibold">Configure Model</h1>
        </div>
        <div className="flex items-center gap-2">
          <IconButton label="Refresh" onClick={() => setNotice("Provider metadata refreshed")}>
            <RefreshCw size={17} />
          </IconButton>
          <IconButton label="More actions" onClick={() => setNotice("More model actions opened")}>
            <MoreHorizontal size={17} />
          </IconButton>
          <Button size="sm" onClick={() => setNotice("Deploy Model queued")}>
            <Zap size={15} />
            Deploy Model
          </Button>
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-6">
          <SurfaceCard>
            <SectionTitle title="Provider Details" />
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Provider">
                <SelectInput value={provider} onChange={(event) => setProvider(event.target.value)}>
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="vertex">Google Vertex AI</option>
                  <option value="cohere">Cohere</option>
                  <option value="custom">Custom Endpoint</option>
                </SelectInput>
              </Field>
              <Field label="Model Identifier">
                <TextInput defaultValue="gpt-4.1-enterprise" placeholder="e.g., gpt-4-turbo" />
              </Field>
            </div>
            <div className="mt-5 grid gap-4">
              <Field label="API Key" hint="Vault Managed">
                <div className="relative">
                  <TextInput className="w-full pr-10 font-mono" type={showKey ? "text" : "password"} defaultValue="sk-prod-vault-managed-key" />
                  <button className="absolute right-2 top-1/2 -translate-y-1/2 rounded-md p-1 text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]" type="button" onClick={() => setShowKey((value) => !value)}>
                    {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
              </Field>
              <Field label="Base URL (Optional)">
                <TextInput defaultValue="https://api.openai.com/v1" />
              </Field>
            </div>
          </SurfaceCard>

          <SurfaceCard>
            <SectionTitle title="Execution Parameters" icon={<SlidersHorizontal size={18} />} />
            <div className="grid gap-5">
              <Field label={`Temperature ${temperature.toFixed(1)}`} hint="Precise (0) to Creative (2)">
                <input className="accent-[var(--color-primary)]" type="range" min={0} max={2} step={0.1} value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} />
              </Field>
              <Field label={`Max Completion Tokens ${tokens.toLocaleString()}`}>
                <input className="accent-[var(--color-primary)]" type="range" min={1024} max={32768} step={1024} value={tokens} onChange={(event) => setTokens(Number(event.target.value))} />
              </Field>
              <Field label="Context Window Limit">
                <SelectInput value={contextWindow} onChange={(event) => setContextWindow(event.target.value)}>
                  <option value="8192">8,192 Tokens</option>
                  <option value="16384">16,384 Tokens</option>
                  <option value="32768">32,768 Tokens</option>
                  <option value="128000">128,000 Tokens</option>
                </SelectInput>
              </Field>
            </div>
          </SurfaceCard>

          <SurfaceCard>
            <SectionTitle title="Access Control" />
            <ToggleSwitch checked={inheritable} label="Inheritable Policy" onCheckedChange={setInheritable} />
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">Allow downstream workspaces and agents to inherit and utilize this model configuration.</p>
          </SurfaceCard>
        </div>

        <div className="space-y-4">
          <SurfaceCard className="sticky top-24">
            <SectionTitle title="Configuration Preview" icon={<Code2 size={18} />} />
            <pre className="max-h-[360px] overflow-auto rounded-lg bg-[var(--color-surface-container-low)] p-4 text-[12px] leading-5">{preview}</pre>
            <div className="mt-5 space-y-3">
              <h3 className="text-sm font-semibold">Estimated Cost</h3>
              {costs.map((line) => (
                <KeyValue key={line.label} label={line.label} value={line.value} />
              ))}
            </div>
            <p className="mt-4 text-[12px] text-[var(--color-on-surface-variant)]">Estimates based on standard public pricing. Negotiated enterprise rates may apply.</p>
            <div className="mt-4">
              <Notice value={notice} />
            </div>
          </SurfaceCard>
        </div>
      </div>
    </TronShell>
  );
}

export function AppsPage() {
  const rows = useMemo(() => getApps(), []);
  const [query, setQuery] = useState("");
  const [source, setSource] = useState("all");
  const [page, setPage] = useState(1);
  const [notice, setNotice] = useState("No app action selected.");
  const filteredRows = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return rows.filter((row) => (source === "all" || row.source === source) && (!normalized || Object.values(row).join(" ").toLowerCase().includes(normalized)));
  }, [query, rows, source]);

  return (
    <TronShell activeItem="Apps" searchPlaceholder="Search apps, paths, worktrees...">
      <PageHeader
        title="Connected Applications"
        description="Manage application sources, paths, worktrees, tags, and quick actions."
        status={`${filteredRows.length} visible`}
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice("Import Apps requested")}>
              <Database size={15} />
              Import
            </Button>
            <Button onClick={() => setNotice("New App requested")}>
              <Plus size={15} />
              New App
            </Button>
          </>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <DataTable<AppRecord>
        title="Application Registry"
        rows={filteredRows}
        rowKey={(row) => row.id}
        toolbar={
          <>
            <TextInput className="h-8 w-56" placeholder="Search apps..." value={query} onChange={(event) => setQuery(event.target.value)} />
            <SelectInput className="h-8 w-40" value={source} onChange={(event) => setSource(event.target.value)}>
              <option value="all">All sources</option>
              <option value="Frontend">Frontend</option>
              <option value="Backend">Backend</option>
              <option value="Worker">Worker</option>
            </SelectInput>
            <Button size="sm" variant="secondary" onClick={() => setNotice("Sort by last update toggled")}>
              <Filter size={14} />
              Sort
            </Button>
          </>
        }
        pagination={
          <div className="flex w-full items-center justify-between">
            <span>
              Page {page} of 2 · {filteredRows.length} matching apps
            </span>
            <div className="flex gap-2">
              <Button size="sm" variant="secondary" disabled={page === 1} onClick={() => setPage(1)}>
                <ChevronLeft size={14} />
                Previous
              </Button>
              <Button size="sm" variant="secondary" disabled={page === 2} onClick={() => setPage(2)}>
                Next
                <ChevronRight size={14} />
              </Button>
            </div>
          </div>
        }
        columns={[
          { id: "name", header: "App Name", render: (row) => <span className="font-medium">{row.name}</span> },
          { id: "source", header: "Source", render: (row) => <Badge tone="accent">{row.source}</Badge> },
          { id: "path", header: "Path", render: (row) => <span className="font-mono text-[13px]">{row.path}</span> },
          { id: "worktree", header: "Worktree", render: (row) => <span>{row.worktree}</span> },
          { id: "tags", header: "Tags", render: (row) => tagList(row.tags) },
          {
            id: "actions",
            header: "Actions",
            align: "right",
            render: (row) => (
              <div className="flex justify-end gap-1">
                <IconButton label={`Edit ${row.name}`} onClick={() => setNotice(`Edit ${row.name}`)}>
                  <Settings2 size={15} />
                </IconButton>
                <IconButton label={`Open terminal for ${row.name}`} onClick={() => setNotice(`Terminal for ${row.name}`)}>
                  <Terminal size={15} />
                </IconButton>
                <IconButton label={`Delete ${row.name}`} onClick={() => setNotice(`Delete ${row.name} requested`)}>
                  <Trash2 size={15} />
                </IconButton>
              </div>
            ),
          },
        ]}
      />
    </TronShell>
  );
}

export function AuditLogsPage() {
  const rows = useMemo(() => getAuditLogs(), []);
  const [tab, setTab] = useState("logs");
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const [page, setPage] = useState(613);
  const [notice, setNotice] = useState("Audit log export is ready.");
  const filteredRows = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return rows.filter((row) => (status === "all" || row.status === status) && (!normalized || Object.values(row).join(" ").toLowerCase().includes(normalized)));
  }, [query, rows, status]);

  return (
    <TronShell activeItem="Audit Logs" searchPlaceholder="Search users, roles, logs...">
      <PageHeader
        title="Users & Roles"
        description="Audit all administrative changes, inherited policy updates, and workspace access events."
        actions={
          <Button variant="secondary" onClick={() => setNotice(`${filteredRows.length} audit rows exported`)}>
            <FileText size={15} />
            Export
          </Button>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <SurfaceCard className="mb-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <SegmentedTabs
            value={tab}
            onValueChange={setTab}
            items={[
              { label: "User Directory", value: "users" },
              { label: "Role Matrix", value: "roles" },
              { label: "Audit Logs", value: "logs" },
            ]}
          />
          <span className="text-sm text-[var(--color-on-surface-variant)]">{tab === "logs" ? "Log table active" : `${tab} tab staged for lead integration`}</span>
        </div>
      </SurfaceCard>
      <DataTable<AuditLogRecord>
        rows={filteredRows}
        rowKey={(row) => row.id}
        toolbar={
          <>
            <TextInput className="h-8 w-60" placeholder="Search audit logs..." value={query} onChange={(event) => setQuery(event.target.value)} />
            <SelectInput className="h-8 w-36" value={status} onChange={(event) => setStatus(event.target.value)}>
              <option value="all">All status</option>
              <option value="Success">Success</option>
              <option value="Warning">Warning</option>
              <option value="Failed">Failed</option>
            </SelectInput>
          </>
        }
        pagination={
          <div className="flex w-full items-center justify-between">
            <span>Showing 1 to {filteredRows.length} of 2,451 entries</span>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="secondary" onClick={() => setPage((value) => Math.max(1, value - 1))}>
                <ChevronLeft size={14} />
              </Button>
              <span className="font-mono text-[13px]">{page}</span>
              <Button size="sm" variant="secondary" onClick={() => setPage((value) => value + 1)}>
                <ChevronRight size={14} />
              </Button>
            </div>
          </div>
        }
        columns={[
          { id: "timestamp", header: "Timestamp", render: (row) => <span className="font-mono text-[13px]">{row.timestamp}</span> },
          { id: "actor", header: "Actor", render: (row) => row.actor },
          { id: "action", header: "Action", render: (row) => row.action },
          { id: "resource", header: "Resource ID", render: (row) => <span className="font-mono text-[13px]">{row.resourceId}</span> },
          { id: "status", header: "Status", render: (row) => <Badge tone={statusTone(row.status)}>{row.status}</Badge> },
          { id: "ip", header: "IP Address", align: "right", render: (row) => <span className="font-mono text-[13px]">{row.ipAddress}</span> },
        ]}
      />
    </TronShell>
  );
}

export function AutomationsPage() {
  const rows = useMemo(() => getAutomations(), []);
  const [selectedId, setSelectedId] = useState(rows[0]?.id ?? "");
  const [tab, setTab] = useState("details");
  const [query, setQuery] = useState("");
  const [notice, setNotice] = useState("Automation inspector ready.");
  const selected = rows.find((row) => row.id === selectedId) ?? rows[0];
  const filteredRows = rows.filter((row) => row.name.toLowerCase().includes(query.trim().toLowerCase()));

  return (
    <TronShell
      activeItem="Automations"
      searchPlaceholder="Search automations..."
      rightPanel={
        selected ? (
          <InspectorPanel title={selected.name}>
            <SegmentedTabs value={tab} onValueChange={setTab} items={[{ label: "Details", value: "details" }, { label: "Logs", value: "logs" }, { label: "Traces", value: "traces" }]} />
            {tab === "details" ? (
              <div className="space-y-4">
                <SurfaceCard className="rounded-xl p-4 shadow-none">
                  <KeyValue label="Schedule" value={selected.cron} />
                  <KeyValue label="Owner" value={selected.owner} />
                  <KeyValue label="Last Run" value={selected.lastRun} />
                </SurfaceCard>
                <SurfaceCard className="rounded-xl p-4 shadow-none">
                  <h3 className="mb-3 text-sm font-semibold">Dependency Flow</h3>
                  <div className="space-y-2">
                    {selected.dependencies.map((dependency) => (
                      <div className="flex items-center gap-2" key={dependency.label}>
                        <span className={cn("size-2 rounded-full", dependency.status === "failed" ? "bg-[var(--color-error)]" : dependency.status === "blocked" ? "bg-[var(--color-warning)]" : "bg-[var(--color-success)]")} />
                        <span className="text-sm">{dependency.label}</span>
                      </div>
                    ))}
                  </div>
                </SurfaceCard>
              </div>
            ) : (
              <SurfaceCard className="rounded-xl p-4 shadow-none">
                {(tab === "logs" ? selected.logs : selected.traces).map((line) => (
                  <p className="font-mono text-[12px] leading-6" key={line}>
                    {line}
                  </p>
                ))}
              </SurfaceCard>
            )}
          </InspectorPanel>
        ) : null
      }
    >
      <PageHeader
        title="Automations Management"
        description="Configure and monitor scheduled intelligence workflows."
        actions={
          <Button onClick={() => setNotice("New automation draft opened")}>
            <Plus size={15} />
            New Automation
          </Button>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <DataTable<AutomationRecord>
        rows={filteredRows}
        rowKey={(row) => row.id}
        toolbar={
          <>
            <TextInput className="h-8 w-56" placeholder="Search automations..." value={query} onChange={(event) => setQuery(event.target.value)} />
            <Button size="sm" variant="secondary" onClick={() => setNotice(`Filter panel opened for ${filteredRows.length} automations`)}>
              <Filter size={14} />
              Filter
            </Button>
          </>
        }
        columns={[
          {
            id: "name",
            header: "Automation",
            render: (row) => (
              <button className="text-left font-medium hover:underline" type="button" onClick={() => setSelectedId(row.id)}>
                {row.name}
              </button>
            ),
          },
          { id: "cadence", header: "Cadence", render: (row) => row.cadence },
          { id: "cron", header: "CRON", render: (row) => <span className="font-mono text-[13px]">{row.cron}</span> },
          { id: "lastRun", header: "Last Run", render: (row) => row.lastRun },
          { id: "status", header: "Status", render: (row) => <Badge tone={statusTone(row.status)}>{row.status}</Badge> },
          {
            id: "actions",
            header: "Actions",
            align: "right",
            render: (row) => (
              <div className="flex justify-end gap-1">
                <IconButton label={`Edit ${row.name}`} onClick={() => setSelectedId(row.id)}>
                  <Settings2 size={15} />
                </IconButton>
                <IconButton label={`Delete ${row.name}`} onClick={() => setNotice(`Delete ${row.name} requested`)}>
                  <Trash2 size={15} />
                </IconButton>
              </div>
            ),
          },
        ]}
      />
    </TronShell>
  );
}

function ChatSessionPage({ variant }: { variant: "reasoning" | "execution" }) {
  const groups = useMemo(() => getChatSessionGroups(), []);
  const messages = useMemo(() => getChatMessages(variant), [variant]);
  const [activeSession, setActiveSession] = useState("primary");
  const [inspectorTab, setInspectorTab] = useState("info");
  const [composer, setComposer] = useState("");
  const [notice, setNotice] = useState("Composer ready.");

  return (
    <TronShell
      activeItem="Chats"
      searchPlaceholder="Search tasks..."
      contentClassName="!p-0"
      rightPanel={
        <InspectorPanel title="Utility Hub">
          <SegmentedTabs value={inspectorTab} onValueChange={setInspectorTab} items={[{ label: "Info", value: "info" }, { label: "Tools", value: "tools" }, { label: "Files", value: "files" }]} />
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            {inspectorTab === "info" ? (
              <div className="space-y-3">
                <KeyValue label="Active Model" value="tron-opus-v2" />
                <KeyValue label="Tokens Used" value="14,285" />
                <KeyValue label="Agents" value="3 active" />
              </div>
            ) : inspectorTab === "tools" ? (
              <div className="space-y-2 text-sm">
                <p>QueryTelemetry</p>
                <p>CodeSearch</p>
                <p>BuildRunner</p>
              </div>
            ) : (
              <div className="space-y-2 text-sm">
                <p>auth/token_validator.go</p>
                <p>deployments/pr-1042.log</p>
              </div>
            )}
          </SurfaceCard>
        </InspectorPanel>
      }
    >
      <div className="grid h-[calc(100vh-65px)] min-h-[720px] grid-cols-[260px_minmax(0,1fr)] bg-white">
        <aside className="border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]">
          <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] p-4">
            <h2 className="text-lg font-semibold">Sessions</h2>
            <IconButton label="New chat" onClick={() => setNotice("New chat requested")}>
              <Plus size={16} />
            </IconButton>
          </div>
          <div className="space-y-5 p-3">
            {groups.map((group) => (
              <section key={group.task}>
                <p className="mb-2 flex items-center gap-2 px-2 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                  <Layers3 size={13} />
                  Task: {group.task}
                </p>
                <div className="space-y-1">
                  {group.sessions.map((session) => (
                    <button className={cn("flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm", activeSession === session.id ? "bg-[var(--color-secondary-container)] font-medium" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]")} type="button" key={session.id} onClick={() => setActiveSession(session.id)}>
                      <ClipboardList size={15} />
                      {session.title}
                    </button>
                  ))}
                </div>
              </section>
            ))}
          </div>
        </aside>

        <section className="flex min-w-0 flex-col">
          <header className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-5 py-4">
            <div>
              <h1 className="text-lg font-semibold">Primary Investigation</h1>
              <p className="text-sm text-[var(--color-on-surface-variant)]">Task: 8842-Alpha · 3 Agents Active · {variant === "reasoning" ? "Reasoning view" : "Execution view"}</p>
            </div>
            <Badge tone="accent">{activeSession}</Badge>
          </header>
          <div className="flex-1 space-y-4 overflow-y-auto bg-[var(--color-surface-container-low)] p-5">
            {messages.map((message) => (
              <ChatBubble key={message.id} message={message} />
            ))}
            <Notice value={notice} />
          </div>
          <div className="border-t border-[var(--color-outline-variant)] bg-white p-4">
            <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3">
              <TextArea className="min-h-20 border-0 bg-transparent p-0 focus:ring-0" placeholder="Shift + Enter to new line" value={composer} onChange={(event) => setComposer(event.target.value)} />
              <div className="mt-3 flex items-center justify-between">
                <div className="flex gap-1">
                  <IconButton label="Attach file" onClick={() => setNotice("Attach file opened")}>
                    <Paperclip size={16} />
                  </IconButton>
                  <IconButton label="Build context" onClick={() => setNotice("Build context requested")}>
                    <Wrench size={16} />
                  </IconButton>
                </div>
                <Button size="sm" onClick={() => setNotice(composer.trim() ? `Queued message: ${composer.trim().slice(0, 40)}` : "Message send requires content")}>
                  <Send size={15} />
                  Send
                </Button>
              </div>
            </div>
          </div>
        </section>
      </div>
    </TronShell>
  );
}

function ChatBubble({ message }: { message: ChatMessage }) {
  const isUser = message.kind === "user";
  return (
    <article className={cn("max-w-3xl rounded-xl border p-4", isUser ? "ml-auto border-[var(--color-primary)] bg-white" : "border-[var(--color-card-border)] bg-white")}>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          {message.kind === "tool" || message.kind === "code" ? <Terminal size={16} /> : isUser ? <CircleCheck size={16} /> : <Bot size={16} />}
          <span className="text-sm font-semibold">{message.actor}</span>
          {message.meta ? <Badge tone="neutral">{message.meta}</Badge> : null}
        </div>
        <span className="text-[12px] text-[var(--color-on-surface-variant)]">{message.time}</span>
      </div>
      <pre className="whitespace-pre-wrap font-sans text-sm leading-6 text-[var(--color-on-surface)]">{message.body}</pre>
    </article>
  );
}

export function ChatSessionReasoningPage() {
  return <ChatSessionPage variant="reasoning" />;
}

export function ChatSessionExecutionPage() {
  return <ChatSessionPage variant="execution" />;
}

export function CreateAutomationPage() {
  const [triggerType, setTriggerType] = useState("Scheduled");
  const [cron, setCron] = useState("0 0 * * *");
  const [prereqs, setPrereqs] = useState(["Data Extract Job (ID: 8821)", "Validation Check (ID: 9012)"]);
  const [notice, setNotice] = useState("Parsed as CRON: 0 0 * * *");

  return (
    <TronShell activeItem="Automations" searchPlaceholder="Search automations...">
      <PageHeader
        breadcrumbs={[{ label: "Automations", href: "/automations" }, { label: "New" }]}
        title="Create Automation"
        actions={
          <Button onClick={() => setNotice("Create automation submitted")}>
            <Plus size={15} />
            Create
          </Button>
        }
      />
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <SurfaceCard>
            <SectionTitle title="Basic Info" />
            <div className="grid gap-4">
              <Field label="Automation Name">
                <TextInput placeholder="e.g., Daily Sync Job" />
              </Field>
              <Field label="Description (Optional)">
                <TextArea placeholder="Describe what this automation does..." />
              </Field>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <SectionTitle title="Trigger Configuration" icon={<Zap size={17} />} />
            <div className="grid gap-4">
              <Field label="Trigger Type">
                <SelectInput value={triggerType} onChange={(event) => setTriggerType(event.target.value)}>
                  <option>Scheduled</option>
                  <option>Webhook</option>
                  <option>Event-driven</option>
                </SelectInput>
              </Field>
              <Field label="Schedule Definition">
                <div className="flex gap-2">
                  <TextInput className="flex-1 font-mono" value={cron} onChange={(event) => setCron(event.target.value)} />
                  <Button variant="secondary" onClick={() => setNotice(`Parsed as CRON: ${cron}`)}>
                    Parse
                  </Button>
                </div>
              </Field>
            </div>
          </SurfaceCard>
          <SurfaceCard>
            <SectionTitle title="Application & Agent" icon={<Database size={17} />} />
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Target Application">
                <SelectInput defaultValue="">
                  <option value="" disabled>
                    Select App...
                  </option>
                  <option>Salesforce Data Sync</option>
                  <option>AWS S3 Bucket Monitor</option>
                  <option>Internal HR Portal</option>
                </SelectInput>
              </Field>
              <Field label="Assigned Agent">
                <SelectInput defaultValue="">
                  <option value="" disabled>
                    Select Agent...
                  </option>
                  <option>Agent Alpha (US-East)</option>
                  <option>Agent Beta (EU-West)</option>
                  <option>Global Pool</option>
                </SelectInput>
              </Field>
            </div>
          </SurfaceCard>
        </div>
        <SurfaceCard>
          <SectionTitle title="Dependencies" icon={<GitBranch size={17} />} />
          <div className="space-y-5">
            <div>
              <div className="mb-2 flex items-center justify-between">
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Pre-requisite Tasks</p>
                <Button size="sm" variant="secondary" onClick={() => setPrereqs((current) => [...current, `New prerequisite ${current.length + 1}`])}>
                  <Plus size={14} />
                  Add Task
                </Button>
              </div>
              <div className="space-y-2">
                {prereqs.map((task) => (
                  <div className="flex items-center justify-between rounded-lg border border-[var(--color-outline-variant)] px-3 py-2 text-sm" key={task}>
                    <span>{task}</span>
                    <IconButton label={`Remove ${task}`} onClick={() => setPrereqs((current) => current.filter((item) => item !== task))}>
                      <Trash2 size={14} />
                    </IconButton>
                  </div>
                ))}
              </div>
            </div>
            <Button variant="secondary" onClick={() => setNotice("Post-execution action added")}>
              <Plus size={14} />
              Add Action
            </Button>
            <Notice value={`${triggerType}. ${notice}`} />
          </div>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}

export function CreatePromptPage() {
  const variables = useMemo(() => getPromptVariables(), []);
  const [tab, setTab] = useState("prompts");
  const [navItem, setNavItem] = useState("Library");
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [visibility, setVisibility] = useState("Workspace");
  const [temperature, setTemperature] = useState(0.7);
  const [notice, setNotice] = useState("2 Detected variables.");
  const toggleNotifications = () => {
    const nextOpen = !notificationsOpen;
    setNotificationsOpen(nextOpen);
    setNotice(nextOpen ? "Notifications panel opened" : "Notifications closed");
  };

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <aside className="fixed inset-y-0 left-0 z-30 w-[280px] border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
        <div className="mb-4 border-b border-[var(--color-outline-variant)] px-3 py-4">
          <h1 className="text-base font-black">Tron AI</h1>
          <p className="text-sm text-[var(--color-on-surface-variant)]">Enterprise Workspace</p>
        </div>
        <Button className="mb-4 w-full" onClick={() => setNotice("New project draft opened")}>
          <Plus size={15} />
          New Project
        </Button>
        {promptShellNavItems.map((item) => (
          <button
            aria-pressed={navItem === item}
            className={cn("mb-1 flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm", navItem === item ? "bg-white font-medium shadow-sm" : "text-[var(--color-on-surface-variant)] hover:bg-white")}
            data-selected={navItem === item ? "true" : "false"}
            type="button"
            key={item}
            onClick={() => {
              setNavItem(item);
              setNotice(`${item} navigation selected`);
            }}
          >
            <Layers3 size={16} />
            {item}
          </button>
        ))}
      </aside>
      <header className="fixed left-[280px] right-0 top-0 z-20 flex h-16 items-center justify-between border-b border-[var(--color-outline-variant)] bg-white/90 px-6 backdrop-blur">
        <div className="flex h-full items-center gap-6">
          <TextInput className="h-8 w-60 rounded-full" placeholder="Search..." />
          <SegmentedTabs value={tab} onValueChange={setTab} items={[{ label: "Dashboard", value: "dashboard" }, { label: "Prompts", value: "prompts" }, { label: "Models", value: "models" }, { label: "Datasets", value: "datasets" }, { label: "Logs", value: "logs" }]} />
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <IconButton label="Notifications" onClick={toggleNotifications}>
              <Settings2 size={16} />
            </IconButton>
            {notificationsOpen ? (
              <div className="absolute right-0 top-10 z-40 w-72 rounded-xl border border-[var(--color-outline-variant)] bg-white p-3 text-sm shadow-lg">
                <p className="font-semibold">Notifications</p>
                <p className="mt-1 text-[var(--color-on-surface-variant)]">3 prompt workflow alerts are ready for review.</p>
                <button className="mt-3 rounded-lg bg-[var(--color-primary)] px-3 py-1.5 text-xs font-semibold text-white" type="button" onClick={() => setNotice("Notification center opened")}>
                  Open center
                </button>
              </div>
            ) : null}
          </div>
        </div>
      </header>
      <main className="ml-[280px] pt-16">
        <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] bg-white px-6 py-4">
          <div>
            <button className="mb-2 flex items-center gap-1 text-sm text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]" type="button" onClick={() => setNotice("Back to Prompts requested")}>
              <ArrowLeft size={15} />
              Back to Prompts
            </button>
            <h1 className="text-2xl font-semibold">Create New Prompt</h1>
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => setNotice("Prompt test queued")}>
              <Play size={15} />
              Test Prompt
            </Button>
            <Button onClick={() => setNotice("Prompt saved")}>
              <Save size={15} />
              Save Prompt
            </Button>
          </div>
        </div>
        <div className="grid gap-6 p-6 xl:grid-cols-[minmax(0,1fr)_360px]">
          <div className="space-y-6">
            <SurfaceCard>
              <div className="grid gap-4 md:grid-cols-3">
                <Field label="Prompt Name">
                  <TextInput placeholder="e.g., Customer Support Response Generator" />
                </Field>
                <Field label="Category">
                  <SelectInput defaultValue="System">
                    <option>System</option>
                    <option>Persona</option>
                    <option>Tool</option>
                  </SelectInput>
                </Field>
                <Field label="Visibility">
                  <div className="flex h-10 items-center gap-4">
                    {["Workspace", "Private", "Public"].map((item) => (
                      <label className="flex items-center gap-2 text-sm" key={item}>
                        <input checked={visibility === item} name="visibility" type="radio" onChange={() => setVisibility(item)} />
                        {item}
                      </label>
                    ))}
                  </div>
                </Field>
              </div>
            </SurfaceCard>
            <SurfaceCard>
              <SectionTitle title="Prompt Template" />
              <div className="grid grid-cols-[44px_1fr] overflow-hidden rounded-lg border border-[var(--color-outline-variant)]">
                <div className="bg-[var(--color-surface-container-low)] py-3 text-center font-mono text-[12px] leading-6 text-[var(--color-on-surface-variant)]">1<br />2<br />3<br />4<br />5<br />6<br />7<br />8<br />9<br />10</div>
                <TextArea className="min-h-[260px] rounded-none border-0 font-mono leading-6" defaultValue={"You are a Tron workspace agent.\n\nSummarize {{incident_summary}} for a {{customer_tier}} customer.\n\nReturn actions, risks, and required owner."} />
              </div>
            </SurfaceCard>
          </div>
          <div className="space-y-4">
            <SurfaceCard>
              <SectionTitle title="Model" icon={<SlidersHorizontal size={17} />} />
              <Field label="Model">
                <SelectInput defaultValue="GPT-4o">
                  <option>GPT-4o</option>
                  <option>Claude 3.5 Sonnet</option>
                  <option>Llama 3 70B</option>
                </SelectInput>
              </Field>
              <Field label={`Temperature ${temperature.toFixed(1)}`} hint="Precise to Creative">
                <input className="accent-[var(--color-primary)]" type="range" min={0} max={2} step={0.1} value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} />
              </Field>
              <KeyValue label="Input (Template)" value="~450" />
              <KeyValue label="Max Output" value="2,048" />
            </SurfaceCard>
            <SurfaceCard>
              <SectionTitle title="Detected Variables" icon={<Code2 size={17} />} />
              <div className="space-y-2">
                {variables.map((variable) => (
                  <div className="rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm" key={variable.name}>
                    <div className="flex justify-between">
                      <span className="font-mono">{variable.name}</span>
                      <Badge tone="neutral">{variable.type}</Badge>
                    </div>
                    <TextInput className="mt-2 h-8 w-full" defaultValue={variable.sample} />
                  </div>
                ))}
              </div>
              <div className="mt-4">
                <Notice value={`${notice} Visibility: ${visibility}. Top tab: ${tab}.`} />
              </div>
            </SurfaceCard>
          </div>
        </div>
      </main>
    </div>
  );
}

export function CreateSkillPage() {
  const schema = useMemo(() => getSkillSchemaPreview(), []);
  const [enabled, setEnabled] = useState(true);
  const [inheritable, setInheritable] = useState(true);
  const [notice, setNotice] = useState("Valid JSON Schema");
  const jsonSchema = JSON.stringify({ name: schema.name, parameters: Object.fromEntries(schema.fields.map((field) => [field.name, field.type])) }, null, 2);

  return (
    <TronShell
      activeItem="Skills"
      searchPlaceholder="Search skills..."
      rightPanel={
        <InspectorPanel title="Schema Preview">
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            <div className="flex items-center gap-2 text-sm font-semibold text-[var(--color-success)]">
              <CircleCheck size={16} />
              {notice}
            </div>
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">The schema is properly formatted and ready to be parsed by the agent engine.</p>
          </SurfaceCard>
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold">
              <Wrench size={16} />
              {schema.name}
            </h3>
            {schema.fields.map((field) => (
              <KeyValue key={field.name} label={field.name} value={`${field.type}${field.required ? " required" : ""}`} />
            ))}
          </SurfaceCard>
        </InspectorPanel>
      }
    >
      <PageHeader
        breadcrumbs={[{ label: "Skills", href: "/skills" }, { label: "New" }]}
        title="Create New Skill"
        description="Define a new capability for agents using JSON Schema."
        actions={
          <Button variant="secondary" onClick={() => setNotice("Template loaded")}>
            Load Template
          </Button>
        }
      />
      <div className="mx-auto max-w-3xl space-y-6 pb-24">
        <SurfaceCard>
          <SectionTitle title="General Details" />
          <div className="grid gap-4">
            <Field label="Skill Name">
              <TextInput placeholder="e.g., Get Weather, Search Knowledge Base" />
            </Field>
            <Field label="Description (Optional)">
              <TextArea placeholder="Describe what this skill does and when agents should use it." />
            </Field>
          </div>
        </SurfaceCard>
        <SurfaceCard>
          <SectionTitle title="Tool Definition" />
          <div className="mb-2 flex items-center justify-between">
            <span className="text-sm font-medium">schema.json</span>
            <Button size="sm" variant="secondary" onClick={() => setNotice("Schema copied")}>
              <Copy size={14} />
              Copy
            </Button>
          </div>
          <TextArea className="min-h-[260px] font-mono text-[13px]" defaultValue={jsonSchema} />
        </SurfaceCard>
        <SurfaceCard>
          <SectionTitle title="Configuration" />
          <div className="space-y-4">
            <ToggleSwitch checked={enabled} label="Enable by Default" onCheckedChange={setEnabled} />
            <ToggleSwitch checked={inheritable} label="Inheritable" onCheckedChange={setInheritable} />
          </div>
        </SurfaceCard>
      </div>
      <div className="fixed bottom-0 left-[280px] right-0 z-30 border-t border-[var(--color-outline-variant)] bg-white/95 px-6 py-3 backdrop-blur">
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => setNotice("Skill creation cancelled")}>
            Cancel
          </Button>
          <Button onClick={() => setNotice("Skill creation submitted")}>Create Skill</Button>
        </div>
      </div>
    </TronShell>
  );
}

export function CreateTaskPage() {
  const quickReference = useMemo(() => getTaskQuickReference(), []);
  const [priority, setPriority] = useState("Med");
  const [notice, setNotice] = useState("Draft task ready.");

  return (
    <TronShell activeItem="Tasks" searchPlaceholder="Search tasks...">
      <PageHeader breadcrumbs={[{ label: "Tasks", href: "/tasks" }, { label: "New" }]} title="Create Task" />
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_300px]">
        <div className="space-y-6">
          <Field label="Task Title">
            <TextInput className="w-full" placeholder="e.g., Analyze Q3 User Engagement Metrics" />
          </Field>
          <div className="grid gap-6 xl:grid-cols-3">
            <Field label="Assign Agent">
              <SelectInput defaultValue="">
                <option value="" disabled>
                  Select an agent...
                </option>
                <option>Nexus (Data Analyst)</option>
                <option>Cipher (Security)</option>
                <option>Atlas (Research)</option>
                <option>Unassigned</option>
              </SelectInput>
            </Field>
            <Field label="Associated App">
              <SelectInput defaultValue="">
                <option value="" disabled>
                  Select application...
                </option>
                <option>Snowflake DB</option>
                <option>Salesforce CRM</option>
                <option>Zendesk Support</option>
                <option>GitHub</option>
              </SelectInput>
            </Field>
            <Field label="Priority">
              <div className="grid h-10 grid-cols-3 gap-2">
                {["Low", "Med", "High"].map((item) => (
                  <button className={cn("rounded-lg border border-[var(--color-outline-variant)] text-sm", priority === item ? "bg-[var(--color-surface-variant)] font-medium text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]")} type="button" key={item} onClick={() => setPriority(item)}>
                    {item}
                  </button>
                ))}
              </div>
            </Field>
          </div>
          <SurfaceCard className="p-0">
            <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-4 py-3">
              <div>
                <h2 className="text-base font-semibold">Description</h2>
                <p className="text-[13px] text-[var(--color-on-surface-variant)]">Markdown supported</p>
              </div>
              <div className="flex gap-1">
                {[List, Link, Code2].map((Icon, index) => (
                  <IconButton label={`Editor tool ${index + 1}`} key={index} onClick={() => setNotice(`Editor tool ${index + 1} applied`)}>
                    <Icon size={15} />
                  </IconButton>
                ))}
              </div>
            </div>
            <TextArea className="min-h-[360px] rounded-none border-0 bg-white p-5" placeholder="Describe the work, expected output, context, and constraints..." />
          </SurfaceCard>
        </div>
        <aside className="space-y-4">
          <SurfaceCard>
            <SectionTitle title="Quick Reference" />
            <h3 className="mb-2 text-sm font-semibold">Tips for Agents</h3>
            <ul className="space-y-2 text-sm text-[var(--color-on-surface-variant)]">
              {quickReference.tips.map((tip) => (
                <li key={tip}>{tip}</li>
              ))}
            </ul>
            <h3 className="mb-2 mt-5 text-sm font-semibold">Recent Variables</h3>
            <div className="flex flex-wrap gap-2">
              {quickReference.variables.map((variable) => (
                <Badge key={variable} tone="neutral">
                  {variable}
                </Badge>
              ))}
            </div>
          </SurfaceCard>
          <Notice value={`${notice} Priority: ${priority}.`} />
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => window.location.assign("/tasks")}>
              Cancel
            </Button>
            <Button onClick={() => {
              setNotice("Task creation submitted");
              window.location.assign("/tasks/detail");
            }}>
              <Plus size={15} />
              Create Task
            </Button>
          </div>
        </aside>
      </div>
    </TronShell>
  );
}

export const prototypePagesA = {
  add_model_tron_standardized_navigation: AddModelPage,
  app_management_light: AppsPage,
  audit_logs_tron_standardized_navigation: AuditLogsPage,
  automations_management_light: AutomationsPage,
  chat_sessions_updated_tron_1: ChatSessionReasoningPage,
  chat_sessions_updated_tron_2: ChatSessionExecutionPage,
  create_automation_tron_standardized: CreateAutomationPage,
  create_new_prompt_tron: CreatePromptPage,
  create_new_skill_tron_standardized: CreateSkillPage,
  create_task_full_width_console_layout: CreateTaskPage,
};
