import {
  useMemo,
  useState,
} from "react";
import {
  ArrowLeft,
  BriefcaseBusiness,
  LockKeyhole,
  ShieldCheck,
} from "lucide-react";
import {
  Link,
  useLocation,
} from "react-router";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import { cn } from "@/shared/lib/utils";
import {
  PageHeader,
  SurfaceCard,
  TronShell,
} from "@/shared";

import { useUserList } from "@/domain/admin/hooks";
import type { UserRecord } from "@/domain/admin/model";
import {
  adminUserRoleOptions,
  getStatusTone,
} from "@/domain/admin/constants";

function sortByOptions(values: string[], options: readonly string[]) {
  const knownValues = options.filter((option) => values.includes(option));
  const customValues = values.filter((value) => !options.includes(value));
  return [...knownValues, ...customValues];
}

function DetailField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-1 text-sm font-medium text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

function AssignmentChips({ label, onToggle, options, selected }: { label: string; onToggle: (value: string) => void; options: readonly string[]; selected: string[] }) {
  return (
    <div className="flex flex-wrap gap-2" aria-label={label}>
      {options.map((item) => {
        const active = selected.includes(item);
        return (
          <button
            className={cn(
              "inline-flex h-8 items-center rounded-full border px-3 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
              active
                ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                : "border-[var(--color-outline-variant)] bg-white text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]",
            )}
            type="button"
            key={item}
            aria-pressed={active}
            onClick={() => onToggle(item)}
          >
            {item}
          </button>
        );
      })}
    </div>
  );
}

export function UserDetailsPage() {
  const users = useUserList();
  const { search } = useLocation();
  const requestedUserId = new URLSearchParams(search).get("user");
  const user = users.find((candidate) => candidate.id === requestedUserId) ?? users[0];
  const [roleAssignments, setRoleAssignments] = useState<Record<string, string[]>>({});
  const [workspaceAssignments, setWorkspaceAssignments] = useState<Record<string, string[]>>({});
  const [notice, setNotice] = useState("");
  const roleOptions = adminUserRoleOptions;
  const workspaceOptions = useMemo(
    () => Array.from(new Set(users.flatMap((item) => item.workspaces))).sort(),
    [users],
  );

  if (!user) {
    return (
      <TronShell activeItem="Users" searchPlaceholder="Search users...">
        <PageHeader
          title="User Detail"
          actions={<Button asChild variant="secondary"><Link to="/admin/users"><ArrowLeft size={16} aria-hidden="true" />Back to Users</Link></Button>}
        />
        <SurfaceCard>No user record found.</SurfaceCard>
      </TronShell>
    );
  }

  const assignedRoles = roleAssignments[user.id] ?? user.roles;
  const assignedWorkspaces = workspaceAssignments[user.id] ?? user.workspaces;

  function toggleRoleAssignment(record: UserRecord, value: string) {
    const active = assignedRoles.includes(value);
    if (active && assignedRoles.length === 1) {
      setNotice(`${record.name} must keep at least one role`);
      return;
    }
    const nextRoles = active ? assignedRoles.filter((item) => item !== value) : sortByOptions([...assignedRoles, value], roleOptions);
    setRoleAssignments((current) => ({ ...current, [record.id]: nextRoles }));
    setNotice(`${value} role ${active ? "removed from" : "assigned to"} ${record.name}`);
  }

  function toggleWorkspaceAssignment(record: UserRecord, value: string) {
    const active = assignedWorkspaces.includes(value);
    if (active && assignedWorkspaces.length === 1) {
      setNotice(`${record.name} must stay in at least one workspace`);
      return;
    }
    const nextWorkspaces = active ? assignedWorkspaces.filter((item) => item !== value) : sortByOptions([...assignedWorkspaces, value], workspaceOptions);
    setWorkspaceAssignments((current) => ({ ...current, [record.id]: nextWorkspaces }));
    setNotice(`${value} workspace ${active ? "removed from" : "assigned to"} ${record.name}`);
  }

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users...">
      <PageHeader
        title={user.name}
        status="User detail"
        actions={<Button asChild variant="secondary"><Link to="/admin/users"><ArrowLeft size={16} aria-hidden="true" />Back to Users</Link></Button>}
      />
      <p className="sr-only" aria-live="polite">{notice}</p>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <SurfaceCard>
          <div className="mb-5 flex items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">Basic Information</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{user.email}</p>
            </div>
            <div className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
              <ShieldCheck size={18} aria-hidden="true" />
            </div>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <DetailField label="Full Name" value={user.name} />
            <DetailField label="Email" value={user.email} />
            <div>
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Status</p>
              <div className="mt-1"><Badge tone={getStatusTone(user.status)}>{user.status}</Badge></div>
            </div>
            <DetailField label="Last Active" value={user.lastActive} />
          </div>
        </SurfaceCard>

        <SurfaceCard className="grid gap-5">
          <section>
            <div className="mb-3 flex items-center gap-2">
              <LockKeyhole size={16} aria-hidden="true" />
              <h2 className="text-base font-semibold">Role</h2>
            </div>
            <AssignmentChips label={`Role for ${user.name}`} options={roleOptions} selected={assignedRoles} onToggle={(value) => toggleRoleAssignment(user, value)} />
          </section>
          <section className="border-t border-[var(--color-outline-variant)] pt-5">
            <div className="mb-3 flex items-center gap-2">
              <BriefcaseBusiness size={16} aria-hidden="true" />
              <h2 className="text-base font-semibold">Workspace</h2>
            </div>
            <AssignmentChips label={`Workspace for ${user.name}`} options={workspaceOptions} selected={assignedWorkspaces} onToggle={(value) => toggleWorkspaceAssignment(user, value)} />
          </section>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}
