import {
  useEffect,
  useState,
} from "react";
import {
  Download,
  Eye,
  MoreHorizontal,
  ShieldCheck,
  UserPlus,
} from "lucide-react";
import {
  Link,
  useLocation,
  useNavigate,
} from "react-router";

import { Badge } from "@/shared/ui/Badge";
import {
  Button,
  LinkButton,
} from "@/shared/ui/Button";
import {
  DataTable,
  MetricCard,
  PageHeader,
  SearchField,
  SelectInput,
  TablePagination,
  useLocaleFormatters,
  useLocaleText,
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
import {
  type UserLifecycleResult,
  archiveAdminUser,
  restoreAdminUser,
} from "@/domain/admin/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

const ALL_FILTER = "All";

interface UsersRouteState {
  createdUser?: UserRecord;
  notice?: string;
}

function userDetailPath(userCode: string) {
  return `/admin/users/detail?userCode=${encodeURIComponent(userCode)}`;
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
  const pt = useLocaleText();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const toast = useToast();
  const routeState = location.state as UsersRouteState | null;
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<UserStatus | typeof ALL_FILTER>("Active");
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(15);
  const [exporting, setExporting] = useState(false);
  const [openRowMenu, setOpenRowMenu] = useState<string | null>(null);
  const [processingRow, setProcessingRow] = useState<string | null>(null);
  const dataSource = getServiceDataSource("admin");
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
  const formatTimestamp = (value: string | null) => (value ? formatDateTime(value, {
    dateStyle: "medium",
    timeStyle: "short",
  }) : pt("Never"));
  const formatPhone = (phone: UserRecord["phone"]) => (phone ? `${phone.countryCode} ${phone.number}` : pt("No phone"));

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
      toast.success(pt("{{count}} users exported", { count: formatNumber(users.records.length) }));
    } catch (error) {
      const message = error instanceof Error ? error.message : pt("Users export failed");
      toast.error(message);
    } finally {
      setExporting(false);
    }
  }

  async function handleRowLifecycleAction(row: UserRecord) {
    const action: "archive" | "restore" = row.status === "Active" ? "archive" : "restore";
    const actionFn = action === "archive" ? archiveAdminUser : restoreAdminUser;
    setProcessingRow(row.userCode);
    setOpenRowMenu(null);

    const fallbackResult: UserLifecycleResult = {
      status: action === "archive" ? "Archived" : "Active",
      archivedAt: action === "archive" ? new Date().toISOString() : null,
    };

    try {
      const endpointResult: UserLifecycleResult = dataSource === "api" ? await actionFn(row.userCode) : fallbackResult;

      users.upsertUser({
        ...row,
        status: endpointResult.status ?? row.status,
        archivedAt: endpointResult.archivedAt ?? row.archivedAt,
        updatedAt: endpointResult.updatedAt ?? row.updatedAt,
      });
      toast.success(action === "archive" ? pt("User archived") : pt("User restored"));
    } catch (error) {
      const message = error instanceof Error ? error.message : action === "archive" ? pt("Failed to archive user") : pt("Failed to restore user");
      toast.error(message);
    } finally {
      setProcessingRow(null);
    }
  }

  return (
    <>
      <PageHeader title={pt("Users")} actions={<><Button variant="secondary" disabled={exporting} aria-busy={exporting} onClick={() => void handleExport()}><Download size={16} aria-hidden="true" />{exporting ? pt("Exporting...") : pt("Export")}</Button><Button onClick={() => navigate("/admin/users/new")}><UserPlus size={16} aria-hidden="true" />{pt("Add User")}</Button></>} />
      <div className="mb-5 grid gap-4 lg:grid-cols-2 2xl:grid-cols-4">
        <MetricCard label={pt("Users")} value={formatNumber(statistics.totalUsers)} trend={pt("Persisted user records")} icon={<ShieldCheck size={18} aria-hidden="true" />} />
        <MetricCard label={pt("Active")} value={formatNumber(statistics.activeUsers)} trend={pt("Active status count")} />
        <MetricCard label={pt("Role Assignments")} value={formatNumber(statistics.roleAssignments)} trend={pt("Managed on user detail pages")} />
        <MetricCard label={pt("Space Assignments")} value={formatNumber(statistics.spaceAssignments)} trend={pt("External space-code references")} />
      </div>
      <DataTable<UserRecord>
        title={pt("User Directory")}
        rows={users.records}
        rowKey={(row) => row.userCode}
        toolbar={
          <>
            <SearchField ariaLabel={pt("Search users")} placeholder={pt("Filter users...")} value={query} onChange={(value) => {
              setQuery(value);
              setCurrentPage(1);
            }} />
            <SelectInput className="h-9 min-w-32" aria-label={pt("Filter by status")} value={statusFilter} onChange={(event) => {
              setStatusFilter(event.target.value as UserStatus | typeof ALL_FILTER);
              setCurrentPage(1);
            }}>
              <option value={ALL_FILTER}>{pt("All Status")}</option>
              {statusOptions.map((item) => <option key={item}>{pt(item)}</option>)}
            </SelectInput>
          </>
        }
        pagination={<TablePagination
          endItem={endItem}
          itemLabel={pt("users")}
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
          { id: "name", header: pt("User"), render: (row) => <Link className="font-medium hover:underline" to={userDetailPath(row.userCode)}>{row.displayName}</Link> },
          { id: "contact", header: pt("Contact"), render: (row) => <div className="text-[13px] text-[var(--color-on-surface-variant)]"><div className="font-medium text-[var(--color-on-surface)]">{row.email}</div><div>{formatPhone(row.phone)}</div></div> },
          { id: "codes", header: pt("Codes"), render: (row) => <div className="max-w-64 text-[13px] text-[var(--color-on-surface-variant)]"><div className="font-medium text-[var(--color-on-surface)]">{row.userCode}</div><div>{row.roleCodes.map((code) => pt(getAdminUserRoleLabel(code))).join(", ")}</div><div>{row.spaceCodes.join(", ")}</div></div> },
          { id: "status", header: pt("Status"), render: (row) => <Badge tone={getStatusTone(row.status)}>{pt(row.status)}</Badge> },
          { id: "last", header: pt("Last Active"), render: (row) => formatTimestamp(row.lastActiveAt) },
          {
            id: "details",
            header: "",
            align: "right",
            render: (row) => {
              const isMenuOpen = openRowMenu === row.userCode;
              const isProcessing = processingRow === row.userCode;
              const showArchive = row.status === "Active";
              const showRestore = row.status === "Archived";
              return (
                <div className="relative flex justify-end gap-1">
                  <LinkButton size="icon" to={userDetailPath(row.userCode)} variant="ghost" aria-label={pt("View {{name}} details", { name: row.displayName })}>
                    <Eye size={16} aria-hidden="true" />
                  </LinkButton>
                  <Button size="icon" variant="ghost" aria-label={pt("Open actions for {{name}}", { name: row.displayName })} disabled={isProcessing} onClick={() => setOpenRowMenu((current) => (current === row.userCode ? null : row.userCode))}>
                    <MoreHorizontal size={16} aria-hidden="true" />
                  </Button>
                  {isMenuOpen ? (
                    <div className="absolute right-4 top-12 z-20 w-40 rounded-lg border border-[var(--color-outline-variant)] bg-white p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.12)]" role="menu" aria-label={pt("Actions for {{name}}", { name: row.displayName })}>
                      {showArchive ? <button type="button" role="menuitem" className="block w-full rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container-low)] disabled:cursor-not-allowed disabled:opacity-60" disabled={isProcessing} onClick={() => void handleRowLifecycleAction(row)}>{pt("Archive User")}</button> : null}
                      {showRestore ? <button type="button" role="menuitem" className="block w-full rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container-low)] disabled:cursor-not-allowed disabled:opacity-60" disabled={isProcessing} onClick={() => void handleRowLifecycleAction(row)}>{pt("Restore User")}</button> : null}
                    </div>
                  ) : null}
                </div>
              );
            },
          },
        ]}
      />
    </>
  );
}
