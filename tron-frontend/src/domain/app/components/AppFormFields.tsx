import { Badge } from "@/shared/ui/Badge";
import {
  Field,
  SelectInput,
  TextArea,
  TextInput,
} from "@/shared";

import type {
  AppDetailData,
  AppRuntimeRole,
  CreateAppPayload,
} from "@/domain/app/model";
import {
  appRuntimeRoleTone,
  appStatusTone,
  formatAppTimestamp,
} from "@/domain/app/model";

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
  return <Badge tone={appStatusTone(status)}>{status}</Badge>;
}

export function AppRuntimeRoleBadge({ runtimeRole }: { runtimeRole: AppRuntimeRole }) {
  return <Badge tone={appRuntimeRoleTone(runtimeRole)}>{runtimeRole}</Badge>;
}

export function AppTagList({ tags }: { tags: string[] }) {
  if (!tags.length) {
    return <span className="text-sm text-[var(--color-on-surface-variant)]">No tags</span>;
  }

  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <Badge key={tag} tone="neutral">
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
  const repositoryHint = "Optional repository metadata for this app.";
  const localDirectoryHint = "Absolute path where this app is stored on this machine.";

  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Field label="App Name">
        <TextInput
          aria-label="App name"
          disabled={disabled}
          required
          value={form.name}
          onChange={(event) => onChange("name", event.target.value)}
        />
      </Field>
      <Field label="Runtime Role">
        <SelectInput
          aria-label="Runtime role"
          disabled={disabled}
          value={form.runtimeRole}
          onChange={(event) => onChange("runtimeRole", event.target.value as AppRuntimeRole)}
        >
          <option value="Frontend">Frontend</option>
          <option value="Backend">Backend</option>
          <option value="Mono">Mono</option>
        </SelectInput>
      </Field>
      <Field label="Local Directory" hint={localDirectoryHint}>
        <TextInput
          aria-label="Local directory"
          disabled={disabled}
          placeholder="/workspace/tron/tron-backend"
          required
          value={form.localDirectory}
          onChange={(event) => onChange("localDirectory", event.target.value)}
        />
      </Field>
      <Field label="Repository URL" hint={repositoryHint}>
        <TextInput
          aria-label="Repository URL"
          disabled={disabled}
          placeholder="https://github.com/clouds3n/tron-console"
          value={form.repositoryUrl}
          onChange={(event) => onChange("repositoryUrl", event.target.value)}
        />
      </Field>
      <Field label="Default Branch">
        <TextInput
          aria-label="Default branch"
          disabled={disabled}
          placeholder="master"
          value={form.defaultBranch}
          onChange={(event) => onChange("defaultBranch", event.target.value)}
        />
      </Field>
      <Field label="Tags" hint="Comma-separated tags used for filtering and task selection context.">
        <TextInput
          aria-label="App tags"
          disabled={disabled}
          placeholder="React, Operator"
          value={form.tagsInput}
          onChange={(event) => onChange("tagsInput", event.target.value)}
        />
      </Field>
      <div className="md:col-span-2">
        <Field label="Description">
          <TextArea
            aria-label="App description"
            className="min-h-32"
            disabled={disabled}
            placeholder="Describe the ownership boundary, runtime purpose, and operator context."
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

export function appArchivedLabel(value: string | null) {
  return formatAppTimestamp(value);
}
