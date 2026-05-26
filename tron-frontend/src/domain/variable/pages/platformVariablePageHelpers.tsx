import type { ReactNode } from "react";

import {
  Field,
  TextArea,
  TextInput,
} from "@/shared";

import type { PlatformVariableRecord } from "@/domain/variable/model";

export interface PlatformVariableFormState {
  description: string;
  key: string;
  value: string;
}

export function emptyPlatformVariableForm(): PlatformVariableFormState {
  return {
    description: "",
    key: "",
    value: "",
  };
}

export function platformVariableFormFromRecord(record: PlatformVariableRecord): PlatformVariableFormState {
  return {
    description: record.description ?? "",
    key: record.key,
    value: record.value,
  };
}

export function normalizePlatformVariableDescription(value: string) {
  return value.trim() ? value.trim() : null;
}

export function formatPlatformVariableTimestamp(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function findPlatformVariable(records: PlatformVariableRecord[], variableKey: string) {
  return records.find((record) => record.key === variableKey) ?? null;
}

export function matchesPlatformVariableSearch(record: PlatformVariableRecord, search: string) {
  if (!search.trim()) return true;
  const normalized = search.trim().toLowerCase();
  return [
    record.key,
    record.value,
    record.description ?? "",
  ].join(" ").toLowerCase().includes(normalized);
}

export function ReadValue({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children || "None"}
    </div>
  );
}

export function PlatformVariableFormFields({
  form,
  keyEditable,
  onChange,
}: {
  form: PlatformVariableFormState;
  keyEditable: boolean;
  onChange: (next: PlatformVariableFormState) => void;
}) {
  return (
    <div className="space-y-5">
      <Field
        label="Variable Key"
        hint={keyEditable
          ? "Keys may use letters, digits, underscore, hyphen, or dot."
          : "Renaming is intentionally disabled for stable inheritance."}
      >
        <TextInput
          aria-label="Platform variable key"
          disabled={!keyEditable}
          value={form.key}
          onChange={(event) => onChange({ ...form, key: event.target.value })}
        />
      </Field>
      <Field label="Variable Value" hint="This value becomes the inherited default for all workspaces.">
        <TextArea
          aria-label="Platform variable value"
          className="min-h-44 font-mono"
          value={form.value}
          onChange={(event) => onChange({ ...form, value: event.target.value })}
        />
      </Field>
      <Field label="Description" hint="Optional operator-facing note about how this platform variable is used.">
        <TextArea
          aria-label="Platform variable description"
          className="min-h-24"
          value={form.description}
          onChange={(event) => onChange({ ...form, description: event.target.value })}
        />
      </Field>
    </div>
  );
}
