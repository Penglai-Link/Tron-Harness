import {
  type FormEvent,
  useState,
} from "react";
import { useNavigate } from "react-router";
import { UserPlus } from "lucide-react";

import {
  Field,
  PageHeader,
  SurfaceCard,
  TextInput,
  useToast,
} from "@/shared";
import { Button } from "@/shared/ui/Button";
import {
  createAdminUser,
  type CreateUserPayload,
} from "@/domain/admin/api";
import type { UserRecord } from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

const defaultCreateForm = {
  displayName: "",
  email: "",
  firstName: "",
  lastName: "",
  locale: "en-US",
  phoneCountryCode: "+1",
  phoneNumber: "",
  timezone: "America/Los_Angeles",
  username: "",
};

type CreateUserForm = typeof defaultCreateForm;

function buildDisplayName(firstName: string, lastName: string) {
  return [firstName.trim(), lastName.trim()].filter(Boolean).join(" ");
}

function makeMockId(userCode: string) {
  let hash = 0n;
  for (const character of userCode) {
    hash = (hash * 31n + BigInt(character.charCodeAt(0))) % 1_000_000n;
  }

  return (734_103_002_743_000_000n + hash).toString();
}

function makeMockCreatedUser(payload: CreateUserPayload): UserRecord {
  const generatedCode = payload.userCode?.trim() || `USER_${payload.username.toUpperCase().replace(/[^A-Z0-9]/g, "_")}`;
  const timestamp = "2026-05-13T00:00:00.000Z";
  return {
    ...payload,
    id: makeMockId(generatedCode),
    userCode: generatedCode,
    sourceType: "Internal",
    avatarUrl: null,
    phone: payload.phone ?? null,
    locale: payload.locale,
    timezone: payload.timezone,
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

  function updateNameField(key: "firstName" | "lastName", value: string) {
    setForm((current) => {
      const nextFirstName = key === "firstName" ? value : current.firstName;
      const nextLastName = key === "lastName" ? value : current.lastName;
      const currentAutoDisplayName = buildDisplayName(current.firstName, current.lastName);
      const shouldSyncDisplayName = current.displayName.trim() === "" || current.displayName === currentAutoDisplayName;

      return {
        ...current,
        [key]: value,
        displayName: shouldSyncDisplayName ? buildDisplayName(nextFirstName, nextLastName) : current.displayName,
      };
    });
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
      locale: form.locale.trim() || "en-US",
      phone: phoneNumber ? { countryCode: form.phoneCountryCode.trim() || "+1", number: phoneNumber } : null,
      roleCodes: [],
      spaceCodes: [],
      timezone: form.timezone.trim() || "America/Los_Angeles",
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
    <>
      <PageHeader
        breadcrumbs={[{ label: "Users", href: "/admin/users" }, { label: "New" }]}
        title="Create User"
        actions={(
          <>
            <Button type="button" variant="secondary" onClick={() => navigate("/admin/users")}>Cancel</Button>
            <Button type="submit" form="create-user-form" disabled={submitting}>
              <UserPlus size={16} aria-hidden="true" />
              {submitting ? "Creating..." : "Create User"}
            </Button>
          </>
        )}
      />
      <SurfaceCard>
        <form className="grid gap-5" id="create-user-form" onSubmit={handleSubmit}>
          <div className="grid gap-4 md:grid-cols-3">
            <Field label="Username">
              <TextInput required value={form.username} onChange={(event) => updateForm("username", event.target.value)} />
            </Field>
            <Field label="First Name">
              <TextInput required value={form.firstName} onChange={(event) => updateNameField("firstName", event.target.value)} />
            </Field>
            <Field label="Last Name">
              <TextInput required value={form.lastName} onChange={(event) => updateNameField("lastName", event.target.value)} />
            </Field>
            <Field label="Display Name">
              <TextInput required value={form.displayName} onChange={(event) => updateForm("displayName", event.target.value)} />
            </Field>
            <Field label="Email">
              <TextInput required type="email" value={form.email} onChange={(event) => updateForm("email", event.target.value)} />
            </Field>
            <Field label="Phone Country Code">
              <TextInput value={form.phoneCountryCode} onChange={(event) => updateForm("phoneCountryCode", event.target.value)} />
            </Field>
            <Field label="Phone">
              <TextInput value={form.phoneNumber} onChange={(event) => updateForm("phoneNumber", event.target.value)} />
            </Field>
            <Field label="Locale">
              <TextInput value={form.locale} onChange={(event) => updateForm("locale", event.target.value)} />
            </Field>
            <Field label="Timezone">
              <TextInput value={form.timezone} onChange={(event) => updateForm("timezone", event.target.value)} />
            </Field>
          </div>
        </form>
      </SurfaceCard>
    </>
  );
}
