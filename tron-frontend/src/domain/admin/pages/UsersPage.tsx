import {
  useEffect,
  useState,
} from "react";
import {
  Download,
  Eye,
  MoreHorizontal,
  Search,
  ShieldCheck,
  UserPlus,
} from "lucide-react";
import {
  Link,
  useLocation,
  useNavigate,
} from "react-router";

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
  useToast,
} from "@/shared";

import {
  useUserList,
  useUserStatistics,
} from "@/domain/admin/hooks";
import type {
  UserRecord,
  UserStatus,
} from "@/domain/admin/model";

import {
  adminUserStatusOptions,
  getAdminUserRoleLabel,
  getStatusTone,
} from "@/domain/admin/constants";

const ALL_FILTER = "All";

interface UsersRouteState {
  createdUser?: UserRecord;
  notice?: string;
}

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

function formatPhone(phone: UserRecord["phone"]) {
  if (!phone) return "No phone";
  return `${phone.countryCode} ${phone.number}`;
}

function csvCell(value: string | readonly string[]) {
  const text = typeof value === "string" ? value : value.join("|");
  return /[",\n]/.test(text) ? `"${text.replaceAll("\"", "\"\"")}"` : text;
}

function downloadUsersCsv(records: UserRecord[]) {
  const headers = ["userCode", "username", "displayName", "email", "status", "roleCodes", "spaceCodes"] as const;
  const rows = records.map((user) => [
    user.userCode,
    user.username,
    user.displayName,
    user.email,
    user.status,
    user.roleCodes,
    user.spaceCodes,
  ].map(csvCell).join(","));
  const blob = new Blob([[headers.join(","), ...rows].join("\n")], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `users-${new Date().toISOString().slice(0, 10)}.csv`;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function UsersPage() {
  const location = useLocation();
  const navigate = useNavigate();
  const toast = useToast();
  const routeState = location.state as UsersRouteState | null;
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<UserStatus | typeof ALL_FILTER>("Active");
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(15);
  const [exporting, setExporting] = useState(false);
  const users = useUserList({
    current: currentPage,
    search: query,
    size: pageSize,
    status: statusFilter,
  });
  const statistics = useUserStatistics();
  const statusOptions = adminUserStatusOptions;
  const pageCount = Math.max(1, Math.ceil(users.total / users.size));
  const startItem = users.total ? (users.current - 1) * users.size + 1 : 0;
  const endItem = users.total ? Math.min(startItem + users.records.length - 1, users.total) : 0;

  useEffect(() => {
    if (routeState?.createdUser) users.upsertUser(routeState.createdUser);
    if (routeState?.notice) toast.success(routeState.notice);
  }, [routeState?.createdUser, routeState?.notice, toast, users.upsertUser]);

  async function handleExport() {
    if (exporting) return;

    setExporting(true);

    try {
      await new Promise((resolve) => window.setTimeout(resolve, 350));
      downloadUsersCsv(users.records);
      toast.success(`${users.records.length} users exported`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Users export failed";
      toast.error(message);
    } finally {
      setExporting(false);
    }
  }

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users...">
      <PageHeader title="Users" actions={<><Button variant="secondary" disabled={exporting} aria-busy={exporting} onClick={() => void handleExport()}><Download size={16} aria-hidden="true" />{exporting ? "Exporting..." : "Export"}</Button><Button onClick={() => navigate("/admin/users/new")}><UserPlus size={16} aria-hidden="true" />Add User</Button></>} />
      <div className="mb-5 grid gap-4 lg:grid-cols-2 2xl:grid-cols-4">
        <MetricCard label="Users" value={String(statistics.totalUsers)} trend="Persisted user records" icon={<ShieldCheck size={18} aria-hidden="true" />} />
        <MetricCard label="Active" value={String(statistics.activeUsers)} trend="Active status count" />
        <MetricCard label="Role Assignments" value={String(statistics.roleAssignments)} trend="Managed on user detail pages" />
        <MetricCard label="Space Assignments" value={String(statistics.spaceAssignments)} trend="External space-code references" />
      </div>
      <DataTable<UserRecord>
        title="User Directory"
        rows={users.records}
        rowKey={(row) => row.userCode}
        toolbar={
          <>
            <SearchField placeholder="Filter users..." value={query} onChange={(value) => {
              setQuery(value);
              setCurrentPage(1);
            }} />
            <SelectInput className="h-9 min-w-32" aria-label="Filter by status" value={statusFilter} onChange={(event) => {
              setStatusFilter(event.target.value as UserStatus | typeof ALL_FILTER);
              setCurrentPage(1);
            }}>
              <option value={ALL_FILTER}>All Status</option>
              {statusOptions.map((item) => <option key={item}>{item}</option>)}
            </SelectInput>
          </>
        }
        pagination={<TablePagination
          endItem={endItem}
          itemLabel="users"
          onPageChange={setCurrentPage}
          onPageSizeChange={(value) => {
            setPageSize(value);
            setCurrentPage(1);
          }}
          page={users.current}
          pageCount={pageCount}
          pageSize={users.size}
          startItem={startItem}
          totalItems={users.total}
        />}
        columns={[
          { id: "name", header: "User", render: (row) => <Link className="font-medium hover:underline" to={userDetailPath(row.userCode)}>{row.displayName}</Link> },
          { id: "contact", header: "Contact", render: (row) => <div className="text-[13px] text-[var(--color-on-surface-variant)]"><div className="font-medium text-[var(--color-on-surface)]">{row.email}</div><div>{formatPhone(row.phone)}</div></div> },
          { id: "codes", header: "Codes", render: (row) => <div className="max-w-64 text-[13px] text-[var(--color-on-surface-variant)]"><div className="font-medium text-[var(--color-on-surface)]">{row.userCode}</div><div>{row.roleCodes.map(getAdminUserRoleLabel).join(", ")}</div><div>{row.spaceCodes.join(", ")}</div></div> },
          { id: "status", header: "Status", render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge> },
          { id: "last", header: "Last Active", render: (row) => formatTimestamp(row.lastActiveAt) },
          { id: "details", header: "", align: "right", render: (row) => <div className="flex justify-end gap-1"><Button asChild size="icon" variant="ghost" aria-label={`View ${row.displayName} details`}><Link to={userDetailPath(row.userCode)}><Eye size={16} aria-hidden="true" /></Link></Button><Button size="icon" variant="ghost" aria-label={`Open actions for ${row.displayName}`} onClick={() => toast.info(`${row.displayName} action menu opened`)}><MoreHorizontal size={16} aria-hidden="true" /></Button></div> },
        ]}
      />
    </TronShell>
  );
}
