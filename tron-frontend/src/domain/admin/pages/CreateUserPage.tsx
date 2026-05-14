import {
  type FormEvent,
  useState,
} from "react";
import { useNavigate } from "react-router";
import { UserPlus } from "lucide-react";

import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextInput,
  TronShell,
  useToast,
} from "@/shared";
import { Button } from "@/shared/ui/Button";
import {
  createAdminUser,
  type CreateUserPayload,
} from "@/domain/admin/api";
import {
  adminUserRoleOptions,
  getAdminUserRoleLabel,
} from "@/domain/admin/constants";
import type {
  UserRecord,
  UserRoleCode,
} from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

const defaultCreateForm = {
  displayName: "",
  email: "",
  firstName: "",
  lastName: "",
  phoneCountryCode: "+1",
  phoneNumber: "",
  roleCode: "VIEWER" as UserRoleCode,
  spaceCodes: "PLATFORM",
  userCode: "",
  username: "",
};

type CreateUserForm = typeof defaultCreateForm;

function splitCodes(value: string) {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
}

function makeMockId(userCode: string) {
  let hash = 0n;
  for (const character of userCode) {
    hash = (hash * 31n + BigInt(character.charCodeAt(0))) % 1_000_000n;
  }

  return (734_103_002_743_000_000n + hash).toString();
}

function makeMockCreatedUser(payload: CreateUserPayload): UserRecord {
  const timestamp = "2026-05-13T00:00:00.000Z";
  return {
    ...payload,
    id: makeMockId(payload.userCode),
    avatarUrl: null,
    phone: payload.phone ?? null,
    locale: "en-US",
    timezone: "UTC",
    status: "Active",
    passwordResetRequired: true,
    lastActiveAt: null,
    createdAt: timestamp,
    updatedAt: timestamp,
    archivedAt: null,
  };
}

export function CreateUserPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const [form, setForm] = useState<CreateUserForm>(defaultCreateForm);
  const [submitting, setSubmitting] = useState(false);

  function updateForm<K extends keyof CreateUserForm>(key: K, value: CreateUserForm[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);

    const phoneNumber = form.phoneNumber.trim();
    const payload: CreateUserPayload = {
      displayName: form.displayName,
      email: form.email,
      firstName: form.firstName,
      lastName: form.lastName,
      phone: phoneNumber ? { countryCode: form.phoneCountryCode.trim() || "+1", number: phoneNumber } : null,
      roleCodes: [form.roleCode],
      spaceCodes: splitCodes(form.spaceCodes),
      userCode: form.userCode,
      username: form.username,
    };

    try {
      const createdUser = getServiceDataSource("admin") === "api"
        ? await createAdminUser(payload)
        : makeMockCreatedUser(payload);
      const notice = `${createdUser.displayName} created successfully`;

      toast.success(notice);
      navigate("/admin/users", {
        state: {
          createdUser,
        },
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : "User could not be created";
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <TronShell activeItem="Users" searchPlaceholder="Search users...">
      <PageHeader breadcrumbs={[{ label: "Users", href: "/admin/users" }, { label: "New" }]} title="Create User" />
      <SurfaceCard>
        <form className="grid gap-5" onSubmit={handleSubmit}>
          <div className="grid gap-4 md:grid-cols-3">
            <Field label="User Code">
              <TextInput required value={form.userCode} onChange={(event) => updateForm("userCode", event.target.value)} />
            </Field>
            <Field label="Username">
              <TextInput required value={form.username} onChange={(event) => updateForm("username", event.target.value)} />
            </Field>
            <Field label="Email">
              <TextInput required type="email" value={form.email} onChange={(event) => updateForm("email", event.target.value)} />
            </Field>
            <Field label="First Name">
              <TextInput required value={form.firstName} onChange={(event) => updateForm("firstName", event.target.value)} />
            </Field>
            <Field label="Last Name">
              <TextInput required value={form.lastName} onChange={(event) => updateForm("lastName", event.target.value)} />
            </Field>
            <Field label="Display Name">
              <TextInput required value={form.displayName} onChange={(event) => updateForm("displayName", event.target.value)} />
            </Field>
            <Field label="Phone Country Code">
              <TextInput value={form.phoneCountryCode} onChange={(event) => updateForm("phoneCountryCode", event.target.value)} />
            </Field>
            <Field label="Phone Number">
              <TextInput value={form.phoneNumber} onChange={(event) => updateForm("phoneNumber", event.target.value)} />
            </Field>
            <Field label="Role Code">
              <SelectInput value={form.roleCode} onChange={(event) => updateForm("roleCode", event.target.value as UserRoleCode)}>
                {adminUserRoleOptions.map((item) => <option key={item} value={item}>{getAdminUserRoleLabel(item)}</option>)}
              </SelectInput>
            </Field>
            <Field label="Space Codes" hint="Comma separated">
              <TextInput value={form.spaceCodes} onChange={(event) => updateForm("spaceCodes", event.target.value)} />
            </Field>
          </div>
          <div className="flex flex-wrap justify-end gap-2">
            <Button type="button" variant="secondary" onClick={() => navigate("/admin/users")}>Cancel</Button>
            <Button type="submit" disabled={submitting}>
              <UserPlus size={16} aria-hidden="true" />
              {submitting ? "Creating..." : "Create User"}
            </Button>
          </div>
        </form>
      </SurfaceCard>
    </TronShell>
  );
}
