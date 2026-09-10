import { Badge } from "@/components/ui/badge";
import { Field, SelectInput, TextArea, TextInput } from "@/components/forms";

import type {
  AppDetailData,
  AppRuntimeRole,
  CreateAppPayload,
} from "@/domain/app/model";
import { useTranslation } from "react-i18next";
import {
  appRuntimeRoleTone,
  appStatusTone,
  formatAppTimestamp,
} from "@/domain/app/model";
import { SelectItem } from "@/components/ui/select";

export interface AppFormState {
  defaultBranch: string;
  description: string;
  localDirectory: string;
  name: string;
  repositoryUrl: string;
  runtimeRole: AppRuntimeRole;
  tagsInput: string;
}

export function emptyAppForm(): AppFormState {
  return {
    defaultBranch: "master",
    description: "",
    localDirectory: "",
    name: "",
    repositoryUrl: "",
    runtimeRole: "Frontend",
    tagsInput: "",
  };
}

export function formFromApp(app: AppDetailData): AppFormState {
  return {
    defaultBranch: app.defaultBranch ?? "",
    description: app.description ?? "",
    localDirectory: app.localDirectory,
    name: app.name,
    repositoryUrl: app.repositoryUrl ?? "",
    runtimeRole: app.runtimeRole,
    tagsInput: app.tags.join(", "),
  };
}

function normalizeTags(value: string) {
  return Array.from(new Set(value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)));
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

export function buildAppPayload(form: AppFormState): CreateAppPayload {
  return {
    defaultBranch: optionalText(form.defaultBranch),
    description: optionalText(form.description),
    localDirectory: form.localDirectory.trim(),
    name: form.name.trim(),
    repositoryUrl: optionalText(form.repositoryUrl),
    runtimeRole: form.runtimeRole,
    tags: normalizeTags(form.tagsInput),
  };
}

export function canSubmitAppForm(form: AppFormState) {
  return Boolean(
    form.name.trim()
    && form.localDirectory.trim(),
  );
}

export function AppStatusBadge({ status }: { status: AppDetailData["status"] }) {
  const { t } = useTranslation("pages");
  return <Badge variant={appStatusTone(status)}>{t(`apps.status.${status}`)}</Badge>;
}

export function AppRuntimeRoleBadge({ runtimeRole }: { runtimeRole: AppRuntimeRole }) {
  const { t } = useTranslation("pages");
  return <Badge variant={appRuntimeRoleTone(runtimeRole)}>{t(`apps.runtimeRole.${runtimeRole}`)}</Badge>;
}

export function AppTagList({ tags }: { tags: string[] }) {
  const { t } = useTranslation("pages");
  if (!tags.length) {
    return <span className="text-sm text-[var(--color-on-surface-variant)]">{t("apps.tags.none")}</span>;
  }

  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <Badge key={tag} variant="info">
          {tag}
        </Badge>
      ))}
    </div>
  );
}

export function AppFormFields({
  disabled = false,
  form,
  onChange,
}: {
  disabled?: boolean;
  form: AppFormState;
  onChange: <K extends keyof AppFormState>(key: K, value: AppFormState[K]) => void;
}) {
  const { t } = useTranslation("pages");
  const pageT = (key: string) => t(`apps.${key}`);
  const repositoryHint = pageT("form.repositoryHint");
  const localDirectoryHint = pageT("form.localDirectoryHint");

  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Field label={pageT("fields.appName")}>
        <TextInput
          aria-label={pageT("aria.appName")}
          disabled={disabled}
          required
          value={form.name}
          onChange={(event) => onChange("name", event.target.value)}
        />
      </Field>
      <Field label={pageT("fields.runtimeRole")}>
        <SelectInput
          aria-label={pageT("aria.runtimeRole")}
          disabled={disabled}
          value={form.runtimeRole}
          onValueChange={(event) => onChange("runtimeRole", event as AppRuntimeRole)}
        >
          <SelectItem value="Frontend">{t("apps.runtimeRole.Frontend")}</SelectItem>
          <SelectItem value="Backend">{t("apps.runtimeRole.Backend")}</SelectItem>
          <SelectItem value="Mono">{t("apps.runtimeRole.Mono")}</SelectItem>
        </SelectInput>
      </Field>
      <Field label={pageT("fields.localDirectory")} hint={localDirectoryHint}>
        <TextInput
          aria-label={pageT("aria.localDirectory")}
          disabled={disabled}
          placeholder={pageT("form.localDirectoryPlaceholder")}
          required
          value={form.localDirectory}
          onChange={(event) => onChange("localDirectory", event.target.value)}
        />
      </Field>
      <Field label={pageT("fields.repositoryUrl")} hint={repositoryHint}>
        <TextInput
          aria-label={pageT("aria.repositoryUrl")}
          disabled={disabled}
          placeholder={pageT("form.repositoryPlaceholder")}
          value={form.repositoryUrl}
          onChange={(event) => onChange("repositoryUrl", event.target.value)}
        />
      </Field>
      <Field label={pageT("fields.defaultBranch")}>
        <TextInput
          aria-label={pageT("aria.defaultBranch")}
          disabled={disabled}
          placeholder="master"
          value={form.defaultBranch}
          onChange={(event) => onChange("defaultBranch", event.target.value)}
        />
      </Field>
      <Field label={pageT("fields.tags")} hint={pageT("form.tagsHint")}>
        <TextInput
          aria-label={pageT("aria.appTags")}
          disabled={disabled}
          placeholder={pageT("form.tagsPlaceholder")}
          value={form.tagsInput}
          onChange={(event) => onChange("tagsInput", event.target.value)}
        />
      </Field>
      <div className="md:col-span-2">
        <Field label={pageT("fields.description")}>
          <TextArea
            aria-label={pageT("aria.appDescription")}
            className="min-h-32"
            disabled={disabled}
            placeholder={pageT("form.descriptionPlaceholder")}
            value={form.description}
            onChange={(event) => onChange("description", event.target.value)}
          />
        </Field>
      </div>
    </div>
  );
}

export function AppDetailField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-1 break-words text-sm font-medium text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

export function appArchivedLabel(value: string | null, locale: string, emptyLabel: string) {
  return formatAppTimestamp(value, locale, emptyLabel);
}
