import {
  type FormEvent,
  useMemo,
  useState,
} from "react";
import {
  Archive,
  ArrowLeft,
  BriefcaseBusiness,
  Copy,
  KeyRound,
  Pencil,
  RotateCcw,
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
  Field,
  PageHeader,
  SurfaceCard,
  TextInput,
  TronShell,
} from "@/shared";

import {
  archiveAdminUser,
  replaceAdminUserRoles,
  replaceAdminUserSpaces,
  resetAdminUserPassword,
  restoreAdminUser,
  updateAdminUserProfile,
  type ReplaceRolesResult,
  type ReplaceSpacesResult,
  type UpdateUserProfilePayload,
} from "@/domain/admin/api";
import { useUserList } from "@/domain/admin/hooks";
import type {
  UserRecord,
  UserRoleCode,
  UserStatus,
} from "@/domain/admin/model";
import {
  adminUserRoleOptions,
  getAdminUserRoleLabel,
  getStatusTone,
} from "@/domain/admin/constants";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

type ResetPasswordState =
  | { mode: "confirm"; userCode: string }
  | { mode: "result"; temporaryPassword: string; userCode: string };

function sortByOptions(values: string[], options: readonly string[]) {
  const knownValues = options.filter((option) => values.includes(option));
  const customValues = values.filter((value) => !options.includes(value));
  return [...knownValues, ...customValues];
}

function formatTimestamp(value: string | null) {
  if (!value) return "Never";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function formatPhone(user: UserRecord) {
  if (!user.phone) return "Not set";
  return `${user.phone.countryCode} ${user.phone.number}`;
}

function generateTemporaryPassword(user: UserRecord) {
  const accountSeed = user.userCode.replace(/^USER_/, "").slice(0, 4) || "USER";
  const entropy = Math.random().toString(36).slice(2, 8).toUpperCase();
  return `TRON-${accountSeed}-${entropy}!`;
}

function DetailField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-1 break-words text-sm font-medium text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

function AssignmentChips({
  formatValue = (value) => value,
  label,
  onToggle,
  options,
  selected,
}: {
  formatValue?: (value: string) => string;
  label: string;
  onToggle: (value: string) => void;
  options: readonly string[];
  selected: string[];
}) {
  return (
    <div className="flex flex-wrap gap-2" aria-label={label}>
      {options.map((item) => {
        const active = selected.includes(item);
        return (
          <button
            className={cn(
              "inline-flex min-h-8 items-center rounded-full border px-3 py-1 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
              active
                ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                : "border-[var(--color-outline-variant)] bg-white text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]",
            )}
            type="button"
            key={item}
            title={item}
            aria-pressed={active}
            onClick={() => onToggle(item)}
          >
            {formatValue(item)}
          </button>
        );
      })}
    </div>
  );
}

function roleCodesFromResult(result: ReplaceRolesResult, fallback: string[]) {
  if (Array.isArray(result.roleCodes)) return result.roleCodes;
  if (Array.isArray(result.roleAssignments)) {
    return result.roleAssignments
      .map((assignment) => assignment.roleCode)
      .filter((roleCode): roleCode is UserRoleCode => Boolean(roleCode));
  }
  return fallback as UserRoleCode[];
}

function spaceCodesFromResult(result: ReplaceSpacesResult, fallback: string[]) {
  if (Array.isArray(result.spaceCodes)) return result.spaceCodes;
  if (Array.isArray(result.spaceAssignments)) {
    return result.spaceAssignments
      .map((assignment) => assignment.spaceCode)
      .filter((spaceCode): spaceCode is string => Boolean(spaceCode));
  }
  return fallback;
}

export function UserDetailsPage() {
  const users = useUserList({ current: 1, size: 100 });
  const { search } = useLocation();
  const requestedUserCode = new URLSearchParams(search).get("userCode");
  const baseUser = requestedUserCode === null
    ? undefined
    : users.records.find((candidate) => candidate.userCode === requestedUserCode);
  const [userOverrides, setUserOverrides] = useState<Record<string, UserRecord>>({});
  const [profileForm, setProfileForm] = useState<UpdateUserProfilePayload | null>(null);
  const [resetState, setResetState] = useState<ResetPasswordState | null>(null);
  const [copyFeedback, setCopyFeedback] = useState("");
  const [notice, setNotice] = useState("");
  const user = baseUser ? userOverrides[baseUser.userCode] ?? baseUser : undefined;
  const roleOptions = adminUserRoleOptions;
  const spaceOptions = useMemo(
    () => Array.from(new Set(users.records.flatMap((item) => item.spaceCodes))).sort(),
    [users.records],
  );
  const isApiMode = getServiceDataSource("admin") === "api";

  function replaceUser(nextUser: UserRecord) {
    setUserOverrides((current) => ({ ...current, [nextUser.userCode]: nextUser }));
  }

  function patchUser(record: UserRecord, patch: Partial<UserRecord>) {
    replaceUser({ ...record, ...patch });
  }

  if (!user) {
    const notFoundMessage = requestedUserCode
      ? `No user record matches ${requestedUserCode}.`
      : "No user record was requested.";

    return (
      <TronShell activeItem="Users" searchPlaceholder="Search users...">
        <PageHeader
          title="User Detail"
          actions={<Button asChild variant="secondary"><Link to="/admin/users"><ArrowLeft size={16} aria-hidden="true" />Back to Users</Link></Button>}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">User not found</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{notFoundMessage}</p>
        </SurfaceCard>
      </TronShell>
    );
  }

  const assignedRoles = user.roleCodes;
  const assignedSpaces = user.spaceCodes;
  const currentStatus = user.status;
  const currentResetState = resetState?.userCode === user.userCode ? resetState : null;

  async function saveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!profileForm || !user) return;

    const updatedUser = isApiMode
      ? await updateAdminUserProfile(user.userCode, profileForm)
      : {
        ...user,
        ...profileForm,
        updatedAt: new Date().toISOString(),
      };

    replaceUser(updatedUser);
    setProfileForm(null);
    setNotice(`${updatedUser.displayName} profile updated`);
  }

  async function toggleRoleAssignment(record: UserRecord, value: string) {
    const active = assignedRoles.includes(value as UserRoleCode);
    const nextRoles = active
      ? assignedRoles.filter((item) => item !== value)
      : sortByOptions([...assignedRoles, value], roleOptions);
    const result = isApiMode
      ? await replaceAdminUserRoles(record.userCode, nextRoles)
      : { roleCodes: nextRoles as UserRoleCode[], updatedAt: new Date().toISOString() };
    const roleCodes = roleCodesFromResult(result, nextRoles);

    patchUser(record, {
      roleCodes,
      updatedAt: result.updatedAt ?? record.updatedAt,
    });
    setNotice(`${getAdminUserRoleLabel(value)} role code ${active ? "removed from" : "assigned to"} ${record.displayName}`);
  }

  async function toggleSpaceAssignment(record: UserRecord, value: string) {
    const active = assignedSpaces.includes(value);
    const nextSpaces = active
      ? assignedSpaces.filter((item) => item !== value)
      : sortByOptions([...assignedSpaces, value], spaceOptions);
    const result = isApiMode
      ? await replaceAdminUserSpaces(record.userCode, nextSpaces)
      : { spaceCodes: nextSpaces, updatedAt: new Date().toISOString() };
    const spaceCodes = spaceCodesFromResult(result, nextSpaces);

    patchUser(record, {
      spaceCodes,
      updatedAt: result.updatedAt ?? record.updatedAt,
    });
    setNotice(`${value} space code ${active ? "removed from" : "assigned to"} ${record.displayName}`);
  }

  async function toggleLifecycle(record: UserRecord) {
    const archiving = currentStatus !== "Archived";
    const result = isApiMode
      ? archiving
        ? await archiveAdminUser(record.userCode)
        : await restoreAdminUser(record.userCode)
      : {
        archivedAt: archiving ? new Date().toISOString() : null,
        status: archiving ? "Archived" as UserStatus : "Active" as UserStatus,
        updatedAt: new Date().toISOString(),
      };
    const nextStatus = result.status ?? (archiving ? "Archived" : "Active");

    patchUser(record, {
      archivedAt: result.archivedAt ?? (nextStatus === "Archived" ? record.archivedAt ?? new Date().toISOString() : null),
      status: nextStatus,
      updatedAt: result.updatedAt ?? record.updatedAt,
    });
    setNotice(`${record.displayName} ${nextStatus === "Archived" ? "archived" : "restored"}`);
  }

  async function confirmPasswordReset(record: UserRecord) {
    const result = isApiMode
      ? await resetAdminUserPassword(record.userCode)
      : { temporaryPassword: generateTemporaryPassword(record), passwordResetRequired: true };
    const temporaryPassword = typeof result.temporaryPassword === "string" && result.temporaryPassword
      ? result.temporaryPassword
      : generateTemporaryPassword(record);

    patchUser(record, {
      passwordResetRequired: result.passwordResetRequired ?? true,
    });
    setResetState({
      mode: "result",
      temporaryPassword,
      userCode: record.userCode,
    });
    setCopyFeedback("");
    setNotice(`Temporary password generated for ${record.displayName}`);
  }

  function copyTemporaryPassword(password: string) {
    void navigator.clipboard?.writeText(password);
    setCopyFeedback("Temporary password copied");
  }

  function closeTemporaryPassword() {
    setResetState(null);
    setCopyFeedback("");
  }

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users...">
      <PageHeader
        title={user.displayName}
        status={user.userCode}
        actions={<Button asChild variant="secondary"><Link to="/admin/users"><ArrowLeft size={16} aria-hidden="true" />Back to Users</Link></Button>}
      />
      <p className="sr-only" aria-live="polite">{notice}</p>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <SurfaceCard>
          <div className="mb-5 flex items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">Basic Information</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{user.username}</p>
            </div>
            <div className="flex items-center gap-2">
              <Badge tone={getStatusTone(currentStatus)}>{currentStatus}</Badge>
              <div className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
                <ShieldCheck size={18} aria-hidden="true" />
              </div>
            </div>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <DetailField label="User Code" value={user.userCode} />
            <DetailField label="Username" value={user.username} />
            <DetailField label="First Name" value={user.firstName} />
            <DetailField label="Last Name" value={user.lastName} />
            <DetailField label="Display Name" value={user.displayName} />
            <DetailField label="Email" value={user.email} />
            <DetailField label="Phone" value={formatPhone(user)} />
            <DetailField label="Locale" value={user.locale} />
            <DetailField label="Timezone" value={user.timezone} />
            <DetailField label="Last Active" value={formatTimestamp(user.lastActiveAt)} />
            <DetailField label="Created" value={formatTimestamp(user.createdAt)} />
            <DetailField label="Updated" value={formatTimestamp(user.updatedAt)} />
          </div>
          <div className="mt-5 flex flex-wrap gap-2 border-t border-[var(--color-outline-variant)] pt-4">
            <Button variant="secondary" onClick={() => setProfileForm({
              displayName: user.displayName,
              email: user.email,
              firstName: user.firstName,
              lastName: user.lastName,
            })}>
              <Pencil size={16} aria-hidden="true" />
              Edit Profile
            </Button>
            <Button variant="secondary" onClick={() => toggleLifecycle(user)}>
              {currentStatus === "Archived" ? <RotateCcw size={16} aria-hidden="true" /> : <Archive size={16} aria-hidden="true" />}
              {currentStatus === "Archived" ? "Restore User" : "Archive User"}
            </Button>
          </div>
          {profileForm ? (
            <form className="mt-5 grid gap-4 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4" onSubmit={saveProfile}>
              <div className="grid gap-4 sm:grid-cols-2">
                <Field label="First Name">
                  <TextInput value={profileForm.firstName} onChange={(event) => setProfileForm((current) => current ? { ...current, firstName: event.target.value } : current)} />
                </Field>
                <Field label="Last Name">
                  <TextInput value={profileForm.lastName} onChange={(event) => setProfileForm((current) => current ? { ...current, lastName: event.target.value } : current)} />
                </Field>
                <Field label="Display Name">
                  <TextInput value={profileForm.displayName} onChange={(event) => setProfileForm((current) => current ? { ...current, displayName: event.target.value } : current)} />
                </Field>
                <Field label="Email">
                  <TextInput type="email" value={profileForm.email} onChange={(event) => setProfileForm((current) => current ? { ...current, email: event.target.value } : current)} />
                </Field>
              </div>
              <div className="flex flex-wrap justify-end gap-2">
                <Button type="button" variant="ghost" onClick={() => setProfileForm(null)}>Cancel</Button>
                <Button type="submit">Save Profile</Button>
              </div>
            </form>
          ) : null}
        </SurfaceCard>

        <div className="grid gap-4">
          <SurfaceCard className="grid gap-5">
            <section>
              <div className="mb-3 flex items-center gap-2">
                <KeyRound size={16} aria-hidden="true" />
                <h2 className="text-base font-semibold">Role Codes</h2>
              </div>
              <AssignmentChips
                label={`Role codes for ${user.displayName}`}
                options={roleOptions}
                selected={assignedRoles}
                formatValue={getAdminUserRoleLabel}
                onToggle={(value) => void toggleRoleAssignment(user, value)}
              />
            </section>
            <section className="border-t border-[var(--color-outline-variant)] pt-5">
              <div className="mb-3 flex items-center gap-2">
                <BriefcaseBusiness size={16} aria-hidden="true" />
                <h2 className="text-base font-semibold">Space Codes</h2>
              </div>
              <AssignmentChips
                label={`Space codes for ${user.displayName}`}
                options={spaceOptions}
                selected={assignedSpaces}
                onToggle={(value) => void toggleSpaceAssignment(user, value)}
              />
            </section>
          </SurfaceCard>

          <SurfaceCard>
            <div className="mb-4 flex items-start justify-between gap-3">
              <div>
                <h2 className="text-base font-semibold">Password Reset</h2>
                <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                  Reset required: {user.passwordResetRequired ? "Yes" : "No"}
                </p>
              </div>
              <Button variant="secondary" onClick={() => setResetState({ mode: "confirm", userCode: user.userCode })}>
                <KeyRound size={16} aria-hidden="true" />
                Reset Password
              </Button>
            </div>

            {currentResetState?.mode === "confirm" ? (
              <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                <p className="text-sm text-[var(--color-on-surface)]">
                  The backend will generate a temporary password and no email verification will be sent.
                </p>
                <div className="mt-3 flex flex-wrap gap-2">
                  <Button size="sm" onClick={() => void confirmPasswordReset(user)}>Confirm Reset</Button>
                  <Button size="sm" variant="ghost" onClick={() => setResetState(null)}>Cancel</Button>
                </div>
              </div>
            ) : null}

            {currentResetState?.mode === "result" ? (
              <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                <p className="text-sm font-medium text-[var(--color-on-surface)]">Temporary password</p>
                <div className="mt-2 flex flex-wrap items-center gap-2">
                  <code className="rounded-md bg-white px-3 py-2 text-sm font-semibold text-[var(--color-on-surface)]">
                    {currentResetState.temporaryPassword}
                  </code>
                  <Button size="sm" variant="secondary" onClick={() => copyTemporaryPassword(currentResetState.temporaryPassword)}>
                    <Copy size={14} aria-hidden="true" />
                    Copy
                  </Button>
                  <Button size="sm" variant="ghost" onClick={closeTemporaryPassword}>Close</Button>
                </div>
                {copyFeedback ? <p className="mt-2 text-sm font-medium text-[var(--color-primary)]">{copyFeedback}</p> : null}
              </div>
            ) : null}
          </SurfaceCard>
        </div>
      </div>
    </TronShell>
  );
}
