import {
  useMemo,
  useState,
} from "react";
import {
  ChevronLeft,
  ChevronRight,
  FileText,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SegmentedTabs,
  SelectInput,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/shared";

import {
  useAuditLogs,
  type AuditLogRecord,
} from "@/domain/admin/api";

function statusTone(status: AuditLogRecord["status"]) {
  if (status === "Success") return "success";
  if (status === "Warning") return "warning";
  if (status === "Failed") return "error";
  return "neutral";
}

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

export function AuditLogsPage() {
  const rows = useAuditLogs();
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
