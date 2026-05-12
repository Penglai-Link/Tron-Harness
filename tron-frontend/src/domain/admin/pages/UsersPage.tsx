import {
  useState,
} from "react";
import {
  Eye,
  MoreHorizontal,
  Search,
  ShieldCheck,
  UserPlus,
} from "lucide-react";
import { Link } from "react-router";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  MetricCard,
  PageHeader,
  SelectInput,
  TablePagination,
  TextInput,
  TronShell,
  useTablePagination,
} from "@/shared";

import { useUserList } from "@/domain/admin/hooks";
import type { UserRecord } from "@/domain/admin/model";

import {
  adminUserStatusOptions,
  getAdminUserRoleLabel,
  getStatusTone,
} from "@/domain/admin/constants";

const ALL_FILTER = "All";

function SearchField({ placeholder, value, onChange }: { placeholder: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput className="h-9 w-full pl-9" placeholder={placeholder} type="search" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function formatTimestamp(value: string | null) {
  if (!value) return "Never";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function userDetailPath(userCode: string) {
  return `/admin/users/detail?userCode=${encodeURIComponent(userCode)}`;
}

export function UsersPage() {
  const users = useUserList();
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("Active");
  const [notice, setNotice] = useState("");
  const statusOptions = adminUserStatusOptions;
  const filtered = users.filter((user) => {
    const searchable = [
      user.userCode,
      user.username,
      user.displayName,
      user.firstName,
      user.lastName,
      user.email,
      user.status,
      ...user.roleCodes,
      ...user.spaceCodes,
    ].join(" ").toLowerCase();

    return (
      (statusFilter === ALL_FILTER || user.status === statusFilter) &&
      searchable.includes(query.trim().toLowerCase())
    );
  });
  const userPagination = useTablePagination(filtered);
  const roleAssignmentCount = users.reduce((total, user) => total + user.roleCodes.length, 0);
  const spaceLinkCount = users.reduce((total, user) => total + user.spaceCodes.length, 0);

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users...">
      <PageHeader title="Users" actions={<><Button variant="secondary" onClick={() => setNotice(`${filtered.length} users exported`)}>Export</Button><Button onClick={() => setNotice("Add user drawer opened")}><UserPlus size={16} aria-hidden="true" />Add User</Button></>} />
      <p className="sr-only" aria-live="polite">{notice}</p>
      <div className="mb-5 grid gap-4 lg:grid-cols-2 2xl:grid-cols-4">
        <MetricCard label="Users" value={String(users.length)} trend="Stable user-code records" icon={<ShieldCheck size={18} aria-hidden="true" />} />
        <MetricCard label="Active" value={String(users.filter((user) => user.status === "Active").length)} trend="Default directory view" />
        <MetricCard label="Role Assignments" value={String(roleAssignmentCount)} trend="Managed on user detail pages" />
        <MetricCard label="Space Links" value={String(spaceLinkCount)} trend="External space-code references" />
      </div>
      <DataTable<UserRecord>
        title="User Directory"
        rows={userPagination.visibleItems}
        rowKey={(row) => row.userCode}
        toolbar={
          <>
            <SearchField placeholder="Filter users..." value={query} onChange={(value) => {
              setQuery(value);
              userPagination.paginationProps.onPageChange(1);
            }} />
            <SelectInput className="h-9 min-w-32" aria-label="Filter by status" value={statusFilter} onChange={(event) => {
              setStatusFilter(event.target.value);
              userPagination.paginationProps.onPageChange(1);
            }}>
              <option value={ALL_FILTER}>All Status</option>
              {statusOptions.map((item) => <option key={item}>{item}</option>)}
            </SelectInput>
          </>
        }
        pagination={<TablePagination itemLabel="users" {...userPagination.paginationProps} />}
        columns={[
          { id: "name", header: "User", render: (row) => <div><Link className="font-medium hover:underline" to={userDetailPath(row.userCode)}>{row.displayName}</Link><div className="text-[13px] text-[var(--color-on-surface-variant)]">{row.username} · {row.email}</div></div> },
          { id: "codes", header: "Codes", render: (row) => <div className="max-w-64 text-[13px] text-[var(--color-on-surface-variant)]"><div className="font-medium text-[var(--color-on-surface)]">{row.userCode}</div><div>{row.roleCodes.map(getAdminUserRoleLabel).join(", ")}</div><div>{row.spaceCodes.join(", ")}</div></div> },
          { id: "status", header: "Status", render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge> },
          { id: "last", header: "Last Active", render: (row) => formatTimestamp(row.lastActiveAt) },
          { id: "details", header: "", align: "right", render: (row) => <div className="flex justify-end gap-1"><Button asChild size="icon" variant="ghost" aria-label={`View ${row.displayName} details`}><Link to={userDetailPath(row.userCode)}><Eye size={16} aria-hidden="true" /></Link></Button><Button size="icon" variant="ghost" aria-label={`Open actions for ${row.displayName}`} onClick={() => setNotice(`${row.displayName} action menu opened`)}><MoreHorizontal size={16} aria-hidden="true" /></Button></div> },
        ]}
      />
    </TronShell>
  );
}
