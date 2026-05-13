import {
  type FormEvent,
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
import { Link } from "react-router";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  MetricCard,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  TronShell,
} from "@/shared";

import {
  createAdminUser,
  type CreateUserPayload,
} from "@/domain/admin/api";
import { useUserList } from "@/domain/admin/hooks";
import type {
  UserRecord,
  UserRoleCode,
  UserStatus,
} from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  adminUserRoleOptions,
  adminUserStatusOptions,
  getAdminUserRoleLabel,
  getStatusTone,
} from "@/domain/admin/constants";

const ALL_FILTER = "All";
const defaultCreateForm = {
  displayName: "",
  email: "",
  firstName: "",
  lastName: "",
  roleCode: "VIEWER" as UserRoleCode,
  spaceCodes: "PLATFORM",
  userCode: "",
  username: "",
};

type CreateUserForm = typeof defaultCreateForm;

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

function splitCodes(value: string) {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
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

function makeMockCreatedUser(payload: CreateUserPayload): UserRecord {
  const now = new Date().toISOString();
  return {
    ...payload,
    id: String(Date.now()),
    avatarUrl: null,
    phone: null,
    locale: "en-US",
    timezone: "UTC",
    status: "Active",
    passwordResetRequired: true,
    lastActiveAt: null,
    createdAt: now,
    updatedAt: now,
    archivedAt: null,
  };
}

export function UsersPage() {
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<UserStatus | typeof ALL_FILTER>("Active");
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(15);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [createForm, setCreateForm] = useState<CreateUserForm>(defaultCreateForm);
  const [notice, setNotice] = useState("");
  const users = useUserList({
    current: currentPage,
    search: query,
    size: pageSize,
    status: statusFilter === ALL_FILTER ? "" : statusFilter,
  });
  const statusOptions = adminUserStatusOptions;
  const pageCount = Math.max(1, Math.ceil(users.total / users.size));
  const startItem = users.total ? (users.current - 1) * users.size + 1 : 0;
  const endItem = users.total ? Math.min(startItem + users.records.length - 1, users.total) : 0;
  const roleAssignmentCount = users.records.reduce((total, user) => total + user.roleCodes.length, 0);
  const spaceLinkCount = users.records.reduce((total, user) => total + user.spaceCodes.length, 0);

  function updateCreateForm<K extends keyof CreateUserForm>(key: K, value: CreateUserForm[K]) {
    setCreateForm((current) => ({ ...current, [key]: value }));
  }

  function handleExport() {
    downloadUsersCsv(users.records);
    setNotice(`${users.records.length} users exported`);
  }

  async function handleCreateUser(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const payload: CreateUserPayload = {
      displayName: createForm.displayName,
      email: createForm.email,
      firstName: createForm.firstName,
      lastName: createForm.lastName,
      roleCodes: [createForm.roleCode],
      spaceCodes: splitCodes(createForm.spaceCodes),
      userCode: createForm.userCode,
      username: createForm.username,
    };
    const createdUser = getServiceDataSource("admin") === "api"
      ? await createAdminUser(payload)
      : makeMockCreatedUser(payload);

    users.upsertUser(createdUser);
    setCreateForm(defaultCreateForm);
    setIsCreateOpen(false);
    setNotice("User created");
  }

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users...">
      <PageHeader title="Users" actions={<><Button variant="secondary" onClick={handleExport}><Download size={16} aria-hidden="true" />Export</Button><Button onClick={() => setIsCreateOpen((value) => !value)}><UserPlus size={16} aria-hidden="true" />Add User</Button></>} />
      <p className="sr-only" aria-live="polite">{notice}</p>
      {isCreateOpen ? (
        <SurfaceCard className="mb-5">
          <form className="grid gap-4" onSubmit={handleCreateUser}>
            <div className="grid gap-4 md:grid-cols-3">
              <Field label="User Code">
                <TextInput required value={createForm.userCode} onChange={(event) => updateCreateForm("userCode", event.target.value)} />
              </Field>
              <Field label="Username">
                <TextInput required value={createForm.username} onChange={(event) => updateCreateForm("username", event.target.value)} />
              </Field>
              <Field label="Email">
                <TextInput required type="email" value={createForm.email} onChange={(event) => updateCreateForm("email", event.target.value)} />
              </Field>
              <Field label="First Name">
                <TextInput required value={createForm.firstName} onChange={(event) => updateCreateForm("firstName", event.target.value)} />
              </Field>
              <Field label="Last Name">
                <TextInput required value={createForm.lastName} onChange={(event) => updateCreateForm("lastName", event.target.value)} />
              </Field>
              <Field label="Display Name">
                <TextInput required value={createForm.displayName} onChange={(event) => updateCreateForm("displayName", event.target.value)} />
              </Field>
              <Field label="Role Code">
                <SelectInput value={createForm.roleCode} onChange={(event) => updateCreateForm("roleCode", event.target.value as UserRoleCode)}>
                  {adminUserRoleOptions.map((item) => <option key={item} value={item}>{getAdminUserRoleLabel(item)}</option>)}
                </SelectInput>
              </Field>
              <Field label="Space Codes" hint="Comma separated">
                <TextInput value={createForm.spaceCodes} onChange={(event) => updateCreateForm("spaceCodes", event.target.value)} />
              </Field>
            </div>
            <div className="flex flex-wrap justify-end gap-2">
              <Button type="button" variant="ghost" onClick={() => setIsCreateOpen(false)}>Cancel</Button>
              <Button type="submit">Create User</Button>
            </div>
          </form>
        </SurfaceCard>
      ) : null}
      <div className="mb-5 grid gap-4 lg:grid-cols-2 2xl:grid-cols-4">
        <MetricCard label="Users" value={String(users.total)} trend="Stable user-code records" icon={<ShieldCheck size={18} aria-hidden="true" />} />
        <MetricCard label="Active" value={String(users.records.filter((user) => user.status === "Active").length)} trend="Current result records" />
        <MetricCard label="Role Assignments" value={String(roleAssignmentCount)} trend="Managed on user detail pages" />
        <MetricCard label="Space Links" value={String(spaceLinkCount)} trend="External space-code references" />
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
