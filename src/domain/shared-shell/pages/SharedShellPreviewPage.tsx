import {
  useMemo,
  useState,
} from "react";
import {
  Activity,
  ChevronLeft,
  ChevronRight,
  Filter,
  Plus,
  ShieldCheck,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  InspectorPanel,
  MetricCard,
  PageHeader,
  SegmentedTabs,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  ToggleSwitch,
  TronShell,
} from "@/shared";

const rows = [
  { name: "Core Platform", owner: "Workspace", inheritance: "Local", status: "Healthy", usage: "284k" },
  { name: "Agent Hub", owner: "Parent Workspace", inheritance: "Inherited", status: "Review", usage: "119k" },
  { name: "Ops Console", owner: "Workspace", inheritance: "Override", status: "Healthy", usage: "76k" },
];

export function SharedShellPreviewPage() {
  const [tab, setTab] = useState("resources");
  const [enabled, setEnabled] = useState(true);
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(1);
  const [notice, setNotice] = useState("Ready");

  const filteredRows = useMemo(() => {
    const scopedRows = tab === "resources" ? rows : rows.filter((row) => row.inheritance !== "Inherited");
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) {
      return scopedRows;
    }
    return scopedRows.filter((row) => Object.values(row).some((value) => value.toLowerCase().includes(normalizedQuery)));
  }, [query, tab]);

  return (
    <TronShell
      activeItem="Tasks"
      searchPlaceholder="Search tasks, agents, traces..."
      topbarActions={
        <Button variant="secondary" size="sm" onClick={() => setNotice("Top bar filter action opened")}>
          <Filter size={15} aria-hidden="true" />
          Filter
        </Button>
      }
      rightPanel={
        <InspectorPanel title="Resource Pattern">
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            <p className="text-sm font-semibold">Step 2 shared components</p>
            <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">
              Shell, page headers, tables, forms, tabs, badges, toggles, cards, and inspector panels are ready for console-page composition.
            </p>
          </SurfaceCard>
          <div className="space-y-3 rounded-xl bg-[var(--color-surface-container-low)] p-4">
            <ToggleSwitch checked={enabled} label="Inheritance controls enabled" onCheckedChange={setEnabled} />
            <p className="text-[12px] text-[var(--color-on-surface-variant)]">Visible switch state is reserved for Step 3 interaction smoke checks.</p>
          </div>
          <div className="rounded-xl border border-[var(--color-card-border)] p-4">
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Last interaction</p>
            <p className="mt-1 text-sm">{notice}</p>
          </div>
        </InspectorPanel>
      }
    >
      <PageHeader
        breadcrumbs={[
          { label: "Tron", href: "/dashboard" },
          { label: "Shared Shell Preview" },
        ]}
        description="Reusable layout and primitive contracts derived from the PRD, design system, and the full Stitch design scan."
        eyebrow="Frontend onboarding / Step 2"
        status="Shared contract"
        title="Workspace console foundation"
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice("Scan export requested")}>
              Export Scan
            </Button>
            <Button onClick={() => setNotice("New resource command triggered")}>
              <Plus size={16} aria-hidden="true" />
              New Resource
            </Button>
          </>
        }
      />

      <div className="grid gap-4 xl:grid-cols-3">
        <MetricCard icon={<Activity size={18} aria-hidden="true" />} label="Console pages" trend="Routed pages are organized by product area" value="30" />
        <MetricCard icon={<ShieldCheck size={18} aria-hidden="true" />} label="Primary archetypes" trend="Kanban, split-pane, graph, chat, forms, tables" value="9" />
        <MetricCard label="Shell width" trend="Matches design doc fixed agent sidebar" value="280px" />
      </div>

      <div className="mt-6 grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-4">
          <SurfaceCard className="p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h2 className="text-base font-semibold">Common controls</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">Tabs and filters must produce visible state in implemented pages.</p>
              </div>
              <SegmentedTabs
                value={tab}
                onValueChange={(value) => {
                  setTab(value);
                  setPage(1);
                  setNotice(`${value === "resources" ? "Resources" : "Overrides"} tab selected`);
                }}
                items={[
                  { label: "Resources", value: "resources" },
                  { label: "Overrides", value: "overrides" },
                ]}
              />
            </div>
          </SurfaceCard>

          <DataTable
            title="Resource governance table"
            rows={filteredRows}
            rowKey={(row) => row.name}
            toolbar={
              <>
                <TextInput className="h-8 w-56" placeholder="Search resources..." type="search" value={query} onChange={(event) => setQuery(event.target.value)} />
                <Button size="sm" variant="secondary" onClick={() => setNotice(`${filteredRows.length} records match the current filter`)}>
                  Filter
                </Button>
              </>
            }
            pagination={
              <div className="flex w-full flex-wrap items-center justify-between gap-3">
                <span>
                  Page {page} of 2 · {filteredRows.length} matching records
                </span>
                <div className="flex items-center gap-2">
                  <Button size="sm" variant="secondary" disabled={page === 1} onClick={() => setPage(1)}>
                    <ChevronLeft size={14} aria-hidden="true" />
                    Previous
                  </Button>
                  <Button size="sm" variant="secondary" disabled={page === 2} onClick={() => setPage(2)}>
                    Next
                    <ChevronRight size={14} aria-hidden="true" />
                  </Button>
                </div>
              </div>
            }
            columns={[
              { id: "name", header: "Name", render: (row) => <span className="font-medium">{row.name}</span> },
              { id: "owner", header: "Owner", render: (row) => <span className="text-[var(--color-on-surface-variant)]">{row.owner}</span> },
              { id: "inheritance", header: "Inheritance", render: (row) => <Badge tone={row.inheritance === "Override" ? "warning" : "neutral"}>{row.inheritance}</Badge> },
              { id: "usage", header: "Usage", align: "right", render: (row) => <span className="font-mono text-[13px]">{row.usage}</span> },
              { id: "status", header: "Status", align: "right", render: (row) => <Badge tone={row.status === "Healthy" ? "success" : "warning"}>{row.status}</Badge> },
            ]}
          />
        </div>

        <SurfaceCard>
          <h2 className="text-base font-semibold">Form primitives</h2>
          <div className="mt-4 grid gap-4">
            <Field label="Resource Name" hint="Used by create/import flows.">
              <TextInput placeholder="knowledge-sync" />
            </Field>
            <Field label="Scope">
              <SelectInput defaultValue="workspace">
                <option value="workspace">Workspace</option>
                <option value="app">Application</option>
                <option value="global">Global</option>
              </SelectInput>
            </Field>
            <Field label="Notes">
              <TextArea placeholder="Describe the policy, source, or automation behavior..." />
            </Field>
          </div>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}
