import {
  useMemo,
  useState,
} from "react";
import {
  Activity,
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
  TablePagination,
  TextArea,
  TextInput,
  ToggleSwitch,
  TronShell,
  useTablePagination,
  useToast,
} from "@/shared";
import { sharedShellPreviewRows } from "@/domain/shared-shell/constants";
import { AuthUserMenuPanel } from "@/domain/auth";
import { SelectItem } from "@/components/ui/select";


export function SharedShellPreviewPage() {
  const toast = useToast();
  const [tab, setTab] = useState("resources");
  const [enabled, setEnabled] = useState(true);
  const [query, setQuery] = useState("");
  const [notice, setNotice] = useState("Ready");

  const filteredRows = useMemo(() => {
    const scopedRows = tab === "resources" ? sharedShellPreviewRows : sharedShellPreviewRows.filter((row) => row.inheritance !== "Inherited");
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) {
      return scopedRows;
    }
    return scopedRows.filter((row) => Object.values(row).some((value) => value.toLowerCase().includes(normalizedQuery)));
  }, [query, tab]);
  const resourcePagination = useTablePagination(filteredRows);

  return (
    <TronShell
      activeItem="Tasks"
      searchPlaceholder="Search tasks, agents, traces..."
      userMenuContent={<AuthUserMenuPanel />}
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
          { label: "Tron", href: "/apps" },
          { label: "Shared Shell Preview" },
        ]}
        title="Workspace console foundation"
        actions={
          <>
            <Button
              variant="secondary"
              onClick={() => {
                setNotice("Scan export requested");
                toast.success("Scan export requested", { title: "Export queued" });
              }}
            >
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
        <MetricCard label="Shell width" trend="Matches design doc fixed agent sidebar" value="240px" />
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
                  resourcePagination.paginationProps.onPageChange(1);
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
            rows={resourcePagination.visibleItems}
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
              <TablePagination {...resourcePagination.paginationProps} />
            }
            columns={[
              { id: "name", header: "Name", render: (row) => <span className="font-medium">{row.name}</span> },
              { id: "owner", header: "Owner", render: (row) => <span className="text-[var(--color-on-surface-variant)]">{row.owner}</span> },
              { id: "inheritance", header: "Inheritance", render: (row) => <Badge tone={row.inheritance === "Override" ? "warning" : "info"}>{row.inheritance}</Badge> },
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
                <SelectItem value="workspace">Workspace</SelectItem>
                <SelectItem value="app">Application</SelectItem>
                <SelectItem value="global">Global</SelectItem>
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
