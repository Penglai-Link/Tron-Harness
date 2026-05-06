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
import { useNavigate } from "@/lib/client-router";
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
} from "@/services/console/agent-workflow-data";

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

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 text-sm">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="font-mono text-[13px]">{value}</span>
    </div>
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
