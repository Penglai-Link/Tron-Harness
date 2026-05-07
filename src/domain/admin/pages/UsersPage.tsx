import {
  useMemo,
  useState,
} from "react";
import {
  MoreHorizontal,
  Search,
  ShieldCheck,
  UserPlus,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  MetricCard,
  PageHeader,
  SegmentedTabs,
  SelectInput,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/shared";

import { useUsers } from "@/domain/admin/hooks";
import type { UserRecord } from "@/domain/admin/model";


import {
  adminManagementTabs,
  adminUserRoleFilters,
  adminUserRoleOptions,
  getStatusTone,
} from "@/domain/admin/constants";

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

export function UsersPage() {
  const initialUsers = useUsers();
  const [roleOverrides, setRoleOverrides] = useState<Record<string, UserRecord["role"]>>({});
  const [tab, setTab] = useState("users");
  const [query, setQuery] = useState("");
  const [role, setRole] = useState("All");
  const [notice, setNotice] = useState("User directory ready.");
  const users = useMemo(
    () => initialUsers.map((user) => ({ ...user, role: roleOverrides[user.id] ?? user.role })),
    [initialUsers, roleOverrides],
  );
  const filtered = users.filter((user) => (role === "All" || user.role === role) && `${user.name} ${user.email}`.toLowerCase().includes(query.toLowerCase()));

  function updateUserRole(id: string, value: UserRecord["role"]) {
    setRoleOverrides((current) => ({ ...current, [id]: value }));
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
          <SegmentedTabs value={tab} onValueChange={setTab} items={adminManagementTabs} />
          <div className="flex gap-2">
            <SearchField placeholder="Filter users..." value={query} onChange={setQuery} />
            <SelectInput className="h-9" value={role} onChange={(event) => setRole(event.target.value)}>
              {adminUserRoleFilters.map((item) => <option key={item}>{item}</option>)}
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
          { id: "role", header: "Role", render: (row) => <SelectInput className="h-8" value={row.role} onChange={(event) => updateUserRole(row.id, event.target.value as UserRecord["role"])}>{adminUserRoleOptions.map((item) => <option key={item}>{item}</option>)}</SelectInput> },
          { id: "workspace", header: "Workspace", render: (row) => row.workspace },
          { id: "status", header: "Status", render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge> },
          { id: "last", header: "Last Active", render: (row) => row.lastActive },
          { id: "menu", header: "", align: "right", render: (row) => <Button size="icon" variant="ghost" aria-label={`Open actions for ${row.name}`} onClick={() => setNotice(`${row.name} action menu opened`)}><MoreHorizontal size={16} aria-hidden="true" /></Button> },
        ]}
      />
    </TronShell>
  );
}
