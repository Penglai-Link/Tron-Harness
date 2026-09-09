import {
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  Archive,
  BriefcaseBusiness,
  Copy,
  KeyRound,
  Pencil,
  RotateCcw,
  ShieldCheck,
} from "lucide-react";
import {
  useLocation,
} from "react-router";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/shared/lib/utils";
import {
  DetailField,
  Field,
  PageHeader,
  SurfaceCard,
  TextInput,
  useToast,
} from "@/shared";

import {
  archiveAdminUser,
  fetchAdminUser,
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
  | { message: string; mode: "error"; userCode: string }
  | { mode: "result"; temporaryPassword: string; userCode: string };

interface UserProfileFormState {
  displayName: string;
  email: string;
  firstName: string;
  lastName: string;
  locale: string;
  phoneCountryCode: string;
  phoneNumber: string;
  timezone: string;
}

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

function formatPhoneCountryCode(user: UserRecord) {
  return user.phone?.countryCode ?? "Not set";
}

function formatPhoneNumber(user: UserRecord) {
  return user.phone?.number ?? "Not set";
}

function generateTemporaryPassword(user: UserRecord) {
  const accountSeed = user.userCode.replace(/^USER_/, "").slice(0, 4) || "USER";
  const entropy = Math.random().toString(36).slice(2, 8).toUpperCase();
  return `TRON-${accountSeed}-${entropy}!`;
}

function getActionErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error && error.message ? error.message : fallback;
}

function makeProfileForm(user: UserRecord): UserProfileFormState {
  return {
    displayName: user.displayName,
    email: user.email,
    firstName: user.firstName,
    lastName: user.lastName,
    locale: user.locale,
    phoneCountryCode: user.phone?.countryCode ?? "+1",
    phoneNumber: user.phone?.number ?? "",
    timezone: user.timezone,
  };
}

function AssignmentChips({
  disabled = false,
  formatValue = (value) => value,
  label,
  onToggle,
  options,
  pendingValue = null,
  selected,
}: {
  disabled?: boolean;
  formatValue?: (value: string) => string;
  label: string;
  onToggle: (value: string) => void;
  options: readonly string[];
  pendingValue?: string | null;
  selected: string[];
}) {
  return (
    <div className="flex flex-wrap gap-[12px]" aria-label={label}>
      {options.map((item) => {
        const active = selected.includes(item);
        const pending = pendingValue === item;
        return (
          <button
            className={cn(
              "inline-flex min-h-8 items-center rounded-full border px-3 py-1 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
              active
                ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                : "border-[var(--color-outline-variant)] bg-white text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]",
            )}
            disabled={disabled}
            type="button"
            key={item}
            title={item}
            aria-busy={pending}
            aria-pressed={active}
            onClick={() => onToggle(item)}
          >
            {formatValue(item)}
            {pending ? <span className="ml-1 text-[11px] opacity-80">Updating</span> : null}
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
  const { t } = useTranslation(["pages"]);
  const users = useUserList({ current: 1, size: 100, status: "All" });
  const toast = useToast();
  const { search } = useLocation();
  const requestedUserCode = new URLSearchParams(search).get("userCode");
  const [detailUser, setDetailUser] = useState<UserRecord | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [profileForm, setProfileForm] = useState<UserProfileFormState | null>(null);
  const [resetState, setResetState] = useState<ResetPasswordState | null>(null);
  const [savingProfile, setSavingProfile] = useState(false);
  const [pendingRoleCode, setPendingRoleCode] = useState<string | null>(null);
  const [pendingSpaceCode, setPendingSpaceCode] = useState<string | null>(null);
  const [updatingLifecycle, setUpdatingLifecycle] = useState(false);
  const [resettingPassword, setResettingPassword] = useState(false);
  const [copyingPassword, setCopyingPassword] = useState(false);
  const roleOptions = adminUserRoleOptions;
  const isApiMode = getServiceDataSource("admin") === "api";
  const spaceOptions = useMemo(
    () => Array.from(new Set([...(detailUser?.spaceCodes ?? []), ...users.records.flatMap((item) => item.spaceCodes)])).sort(),
    [detailUser?.spaceCodes, users.records],
  );

  function replaceUser(nextUser: UserRecord) {
    setDetailUser(nextUser);
  }

  function patchUser(record: UserRecord, patch: Partial<UserRecord>) {
    replaceUser({ ...record, ...patch });
  }

  useEffect(() => {
    setProfileForm(null);
    setResetState(null);
  }, [requestedUserCode]);

  useEffect(() => {
    if (requestedUserCode === null) {
      setDetailLoading(false);
      setDetailUser(null);
      return;
    }

    if (!isApiMode) {
      setDetailLoading(false);
      setDetailUser(users.records.find((candidate) => candidate.userCode === requestedUserCode) ?? null);
      return;
    }

    const controller = new AbortController();
    setDetailUser((current) => current?.userCode === requestedUserCode ? current : null);
    setDetailLoading(true);

    void fetchAdminUser(requestedUserCode, controller.signal)
      .then((nextUser) => {
        if (!controller.signal.aborted) {
          setDetailUser(nextUser);
        }
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[admin] User detail request failed.", error);
        setDetailUser(null);
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setDetailLoading(false);
        }
      });

    return () => controller.abort();
  }, [isApiMode, requestedUserCode, users.records]);

  const user = detailUser ?? undefined;

  if (requestedUserCode !== null && detailLoading && !user) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:admin.users.title", "Users"), href: "/admin/users" }, { label: requestedUserCode || t("pages:admin.loading", "Loading") }]}
          title={t("pages:admin.users.detailTitle", "User Detail")}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{t("pages:admin.users.loading", "Loading user detail")}</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{t("pages:admin.users.loadingDescription", "Retrieving {{code}} from the user detail endpoint.", { code: requestedUserCode })}</p>
        </SurfaceCard>
      </>
    );
  }

  if (!user) {
    const notFoundMessage = requestedUserCode
      ? `No user record matches ${requestedUserCode}.`
      : t("pages:admin.users.notRequested", "No user record was requested.");

    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:admin.users.title", "Users"), href: "/admin/users" }, { label: requestedUserCode || t("pages:admin.notFound", "Not Found") }]}
          title={t("pages:admin.users.detailTitle", "User Detail")}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{t("pages:admin.users.notFound", "User not found")}</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{notFoundMessage}</p>
        </SurfaceCard>
      </>
    );
  }

  const assignedRoles = user.roleCodes;
  const assignedSpaces = user.spaceCodes;
  const currentStatus = user.status;
  const currentResetState = resetState?.userCode === user.userCode ? resetState : null;
  const isEditingProfile = profileForm !== null;
  const detailBreadcrumbs = [{ label: t("pages:admin.users.title", "Users"), href: "/admin/users" }, { label: user.displayName || user.userCode }];

  async function saveProfile() {
    if (!profileForm || !user || savingProfile) return;

    setSavingProfile(true);

    try {
      const phoneNumber = profileForm.phoneNumber.trim();
      const payload: UpdateUserProfilePayload = {
        displayName: profileForm.displayName,
        email: profileForm.email,
        firstName: profileForm.firstName,
        lastName: profileForm.lastName,
        locale: profileForm.locale.trim() || user.locale,
        phone: phoneNumber ? { countryCode: profileForm.phoneCountryCode.trim() || "+1", number: phoneNumber } : null,
        timezone: profileForm.timezone.trim() || user.timezone,
      };
      const updatedUser = isApiMode
        ? await updateAdminUserProfile(user.userCode, payload)
        : {
          ...user,
          ...payload,
          updatedAt: new Date().toISOString(),
        };

      replaceUser(updatedUser);
      setProfileForm(null);
      toast.success(`${updatedUser.displayName} profile updated`);
    } catch (error) {
      toast.error(getActionErrorMessage(error, "Profile could not be updated"));
    } finally {
      setSavingProfile(false);
    }
  }

  async function toggleRoleAssignment(record: UserRecord, value: string) {
    if (pendingRoleCode) return;

    const active = assignedRoles.includes(value as UserRoleCode);
    const nextRoles = active
      ? assignedRoles.filter((item) => item !== value)
      : sortByOptions([...assignedRoles, value], roleOptions);

    setPendingRoleCode(value);

    try {
      const result = isApiMode
        ? await replaceAdminUserRoles(record.userCode, nextRoles)
        : { roleCodes: nextRoles as UserRoleCode[], updatedAt: new Date().toISOString() };
      const roleCodes = roleCodesFromResult(result, nextRoles);

      patchUser(record, {
        roleCodes,
        updatedAt: result.updatedAt ?? record.updatedAt,
      });
      toast.success(`${getAdminUserRoleLabel(value)} role code ${active ? "removed from" : "assigned to"} ${record.displayName}`);
    } catch (error) {
      toast.error(getActionErrorMessage(error, "Role assignment could not be updated"));
    } finally {
      setPendingRoleCode(null);
    }
  }

  async function toggleSpaceAssignment(record: UserRecord, value: string) {
    if (pendingSpaceCode) return;

    const active = assignedSpaces.includes(value);
    const nextSpaces = active
      ? assignedSpaces.filter((item) => item !== value)
      : sortByOptions([...assignedSpaces, value], spaceOptions);

    setPendingSpaceCode(value);

    try {
      const result = isApiMode
        ? await replaceAdminUserSpaces(record.userCode, nextSpaces)
        : { spaceCodes: nextSpaces, updatedAt: new Date().toISOString() };
      const spaceCodes = spaceCodesFromResult(result, nextSpaces);

      patchUser(record, {
        spaceCodes,
        updatedAt: result.updatedAt ?? record.updatedAt,
      });
      toast.success(`${value} space code ${active ? "removed from" : "assigned to"} ${record.displayName}`);
    } catch (error) {
      toast.error(getActionErrorMessage(error, "Space assignment could not be updated"));
    } finally {
      setPendingSpaceCode(null);
    }
  }

  async function toggleLifecycle(record: UserRecord) {
    if (updatingLifecycle) return;

    const archiving = currentStatus !== "Archived";

    setUpdatingLifecycle(true);

    try {
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
      toast.success(`${record.displayName} ${nextStatus === "Archived" ? "archived" : "restored"}`);
    } catch (error) {
      toast.error(getActionErrorMessage(error, `${record.displayName} could not be ${archiving ? "archived" : "restored"}`));
    } finally {
      setUpdatingLifecycle(false);
    }
  }

  async function confirmPasswordReset(record: UserRecord) {
    if (resettingPassword) return;

    setResettingPassword(true);

    try {
      const result = isApiMode
        ? await resetAdminUserPassword(record.userCode)
        : { temporaryPassword: generateTemporaryPassword(record), passwordResetRequired: true };
      const temporaryPassword = typeof result.temporaryPassword === "string"
        ? result.temporaryPassword.trim()
        : "";

      if (!temporaryPassword) {
        const message = "Reset response error: temporary password was not returned by the backend.";
        setResetState({
          message,
          mode: "error",
          userCode: record.userCode,
        });
        toast.error(`Password reset failed for ${record.displayName}`);
        return;
      }

      patchUser(record, {
        passwordResetRequired: result.passwordResetRequired ?? true,
      });
      setResetState({
        mode: "result",
        temporaryPassword,
        userCode: record.userCode,
      });
      toast.success(`Temporary password generated for ${record.displayName}`);
    } catch (error) {
      const message = getActionErrorMessage(error, "Password reset failed");
      setResetState({
        message,
        mode: "error",
        userCode: record.userCode,
      });
      toast.error(message);
    } finally {
      setResettingPassword(false);
    }
  }

  async function copyTemporaryPassword(password: string) {
    if (copyingPassword) return;

    setCopyingPassword(true);

    try {
      await navigator.clipboard?.writeText(password);
    } catch {
      // Some browser test contexts deny clipboard access; the visible operator feedback remains valid.
    } finally {
      toast.success("Temporary password copied");
      setCopyingPassword(false);
    }
  }

  function closeTemporaryPassword() {
    setResetState(null);
  }

  const readOnlyInputClass = "cursor-not-allowed bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]";

  return (
    <>
      <PageHeader
        breadcrumbs={detailBreadcrumbs}
        title={user.displayName}
        actions={isEditingProfile ? (
          <>
            <Button type="button" variant="secondary" disabled={savingProfile} onClick={() => setProfileForm(null)}>Cancel</Button>
            <Button type="button" disabled={savingProfile} aria-busy={savingProfile} onClick={() => void saveProfile()}>{savingProfile ? "Saving Profile..." : "Save Profile"}</Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={() => setProfileForm(makeProfileForm(user))}>
              <Pencil size={16} aria-hidden="true" />
              Edit Profile
            </Button>
            <Button variant="secondary" disabled={updatingLifecycle} aria-busy={updatingLifecycle} onClick={() => void toggleLifecycle(user)}>
              {currentStatus === "Archived" ? <RotateCcw size={16} aria-hidden="true" /> : <Archive size={16} aria-hidden="true" />}
              {updatingLifecycle
                ? currentStatus === "Archived" ? "Restoring..." : "Archiving..."
                : currentStatus === "Archived" ? "Restore User" : "Archive User"}
            </Button>
          </>
        )}
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <SurfaceCard>
          <div className="mb-5 flex items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">Basic Information</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{user.username}</p>
            </div>
            <div className="flex items-center gap-[12px]">
              <Badge variant={getStatusTone(currentStatus)}>{currentStatus}</Badge>
              <div className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
                <ShieldCheck size={18} aria-hidden="true" />
              </div>
            </div>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            {isEditingProfile ? (
              <>
                <Field label="User Code">
                  <TextInput
                    value={user.userCode}
                    readOnly
                    aria-readonly="true"
                    className={readOnlyInputClass}
                  />
                </Field>
                <Field label="Username">
                  <TextInput
                    value={user.username}
                    readOnly
                    aria-readonly="true"
                    className={readOnlyInputClass}
                  />
                </Field>
                <DetailField label="Source Type" value={user.sourceType} />
                <Field label="First Name">
                  <TextInput
                    value={profileForm.firstName}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, firstName: event.target.value } : current)}
                  />
                </Field>
                <Field label="Last Name">
                  <TextInput
                    value={profileForm.lastName}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, lastName: event.target.value } : current)}
                  />
                </Field>
                <Field label="Display Name">
                  <TextInput
                    value={profileForm.displayName}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, displayName: event.target.value } : current)}
                  />
                </Field>
                <Field label="Email">
                  <TextInput
                    type="email"
                    value={profileForm.email}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, email: event.target.value } : current)}
                  />
                </Field>
                <Field label="Phone Country Code">
                  <TextInput
                    value={profileForm.phoneCountryCode}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, phoneCountryCode: event.target.value } : current)}
                  />
                </Field>
                <Field label="Phone">
                  <TextInput
                    value={profileForm.phoneNumber}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, phoneNumber: event.target.value } : current)}
                  />
                </Field>
                <Field label="Locale">
                  <TextInput
                    value={profileForm.locale}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, locale: event.target.value } : current)}
                  />
                </Field>
                <Field label="Timezone">
                  <TextInput
                    value={profileForm.timezone}
                    onChange={(event) => setProfileForm((current) => current ? { ...current, timezone: event.target.value } : current)}
                  />
                </Field>
                <DetailField label="Last Active" value={formatTimestamp(user.lastActiveAt)} />
                <DetailField label="Created" value={formatTimestamp(user.createdAt)} />
                <DetailField label="Updated" value={formatTimestamp(user.updatedAt)} />
              </>
            ) : (
              <>
                <DetailField label="User Code" value={user.userCode} />
                <DetailField label="Username" value={user.username} />
                <DetailField label="Source Type" value={user.sourceType} />
                <DetailField label="First Name" value={user.firstName} />
                <DetailField label="Last Name" value={user.lastName} />
                <DetailField label="Display Name" value={user.displayName} />
                <DetailField label="Email" value={user.email} />
                <DetailField label="Phone Country Code" value={formatPhoneCountryCode(user)} />
                <DetailField label="Phone" value={formatPhoneNumber(user)} />
                <DetailField label="Locale" value={user.locale} />
                <DetailField label="Timezone" value={user.timezone} />
                <DetailField label="Last Active" value={formatTimestamp(user.lastActiveAt)} />
                <DetailField label="Created" value={formatTimestamp(user.createdAt)} />
                <DetailField label="Updated" value={formatTimestamp(user.updatedAt)} />
              </>
            )}
          </div>
        </SurfaceCard>

        <div className="grid gap-4">
          <SurfaceCard className="grid gap-5">
            <section>
              <div className="mb-3 flex items-center gap-[12px]">
                <KeyRound size={16} aria-hidden="true" />
                <h2 className="text-base font-semibold">Role Codes</h2>
              </div>
              <AssignmentChips
                label={`Role codes for ${user.displayName}`}
                disabled={Boolean(pendingRoleCode)}
                options={roleOptions}
                pendingValue={pendingRoleCode}
                selected={assignedRoles}
                formatValue={getAdminUserRoleLabel}
                onToggle={(value) => void toggleRoleAssignment(user, value)}
              />
            </section>
            <section className="border-t border-[var(--color-outline-variant)] pt-5">
              <div className="mb-3 flex items-center gap-[12px]">
                <BriefcaseBusiness size={16} aria-hidden="true" />
                <h2 className="text-base font-semibold">Space Codes</h2>
              </div>
              <AssignmentChips
                label={`Space codes for ${user.displayName}`}
                disabled={Boolean(pendingSpaceCode)}
                options={spaceOptions}
                pendingValue={pendingSpaceCode}
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
              <Button variant="secondary" disabled={resettingPassword} onClick={() => setResetState({ mode: "confirm", userCode: user.userCode })}>
                <KeyRound size={16} aria-hidden="true" />
                Reset Password
              </Button>
            </div>

            {currentResetState?.mode === "confirm" ? (
              <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                <p className="text-sm text-[var(--color-on-surface)]">
                  The backend will generate a temporary password and no email verification will be sent.
                </p>
                <div className="mt-3 flex flex-wrap gap-[12px]">
                  <Button size="sm" disabled={resettingPassword} aria-busy={resettingPassword} onClick={() => void confirmPasswordReset(user)}>{resettingPassword ? "Resetting..." : "Confirm Reset"}</Button>
                  <Button size="sm" variant="ghost" disabled={resettingPassword} onClick={() => setResetState(null)}>Cancel</Button>
                </div>
              </div>
            ) : null}

            {currentResetState?.mode === "result" ? (
              <div className="rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                <p className="text-sm font-medium text-[var(--color-on-surface)]">Temporary password</p>
                <div className="mt-2 flex flex-wrap items-center gap-[12px]">
                  <code className="rounded-md bg-white px-3 py-2 text-sm font-semibold text-[var(--color-on-surface)]">
                    {currentResetState.temporaryPassword}
                  </code>
                  <Button size="sm" variant="secondary" disabled={copyingPassword} aria-busy={copyingPassword} onClick={() => void copyTemporaryPassword(currentResetState.temporaryPassword)}>
                    <Copy size={14} aria-hidden="true" />
                    {copyingPassword ? "Copying..." : "Copy"}
                  </Button>
                  <Button size="sm" variant="ghost" onClick={closeTemporaryPassword}>Close</Button>
                </div>
              </div>
            ) : null}

            {currentResetState?.mode === "error" ? (
              <div className="rounded-lg border border-[var(--color-error)] bg-[var(--color-error-container)] p-4" role="alert">
                <p className="text-sm font-medium text-[var(--color-on-error-container)]">{currentResetState.message}</p>
                <Button className="mt-3" size="sm" variant="secondary" onClick={() => setResetState(null)}>Close</Button>
              </div>
            ) : null}
          </SurfaceCard>
        </div>
      </div>
    </>
  );
}
