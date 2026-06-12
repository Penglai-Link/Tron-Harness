import type { ReactNode } from "react";

import {
  Field,
  TextArea,
  TextInput,
} from "@/shared";

import type {
  VariableRecord,
  VariableValueMode,
} from "@/domain/variable/model";

import {
  SensitiveDataCheckbox,
  isSensitiveRecord,
} from "./platformVariablePageHelpers";

export interface WorkspaceVariableFormState {
  description: string;
  isSensitive: boolean;
  key: string;
  value: string;
  valueMode: VariableValueMode;
}

export function emptyWorkspaceVariableForm(): WorkspaceVariableFormState {
  return {
    description: "",
    isSensitive: false,
    key: "",
    value: "",
    valueMode: "REPLACE",
  };
}

export function workspaceVariableFormFromRecord(record: VariableRecord): WorkspaceVariableFormState {
  const isSensitive = isSensitiveRecord(record);
  return {
    description: record.description ?? "",
    isSensitive,
    key: record.key,
    value: isSensitive ? "" : record.value,
    valueMode: isSensitive ? "PRESERVE" : "REPLACE",
  };
}

export function workspaceVariableFormFromOverride(record: VariableRecord): WorkspaceVariableFormState {
  const isSensitive = isSensitiveRecord(record);
  return {
    description: record.description ?? "",
    isSensitive,
    key: record.key,
    value: isSensitive ? "" : record.value,
    valueMode: "REPLACE",
  };
}

export function normalizeWorkspaceVariableDescription(value: string) {
  return value.trim() ? value.trim() : null;
}

export function formatWorkspaceVariableTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Unknown";

  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function matchesWorkspaceVariableSearch(record: VariableRecord, search: string) {
  if (!search.trim()) return true;
  const value = [
    record.key,
    isSensitiveRecord(record) ? "" : record.value,
    record.description ?? "",
    record.sourceLabel,
    record.sourceWorkspaceCode,
    record.shadowedWorkspaceName ?? "",
    record.shadowedWorkspaceCode ?? "",
    record.shadowedLabel ?? "",
  ].join(" ").toLowerCase();
  return value.includes(search.trim().toLowerCase());
}

export function getWorkspaceVariableInheritedFrom(record: VariableRecord) {
  if (record.sourceKind === "Local") return null;

  if (record.sourceKind === "Override") {
    if (record.shadowedLabel || record.shadowedWorkspaceName || record.shadowedWorkspaceCode) {
      return {
        code: record.shadowedWorkspaceCode,
        label: record.shadowedLabel?.trim() || record.shadowedWorkspaceName?.trim() || record.shadowedWorkspaceCode || "Workspace",
      };
    }

    return null;
  }

  return {
    code: record.sourceWorkspaceCode,
    label: record.sourceLabel?.trim() || record.sourceWorkspaceName?.trim() || record.sourceWorkspaceCode || "Workspace",
  };
}

export function isWorkspaceVariableEditable(record: VariableRecord | null | undefined) {
  return Boolean(record && (record.sourceKind === "Local" || record.sourceKind === "Override"));
}

export function isWorkspaceVariableRemovable(record: VariableRecord | null | undefined) {
  return isWorkspaceVariableEditable(record);
}

export function ReadValue({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children || "None"}
    </div>
  );
}

export function WorkspaceVariableFormFields({
  disableSensitiveDataToggle = false,
  form,
  keyEditable,
  mode,
  onChange,
}: {
  disableSensitiveDataToggle?: boolean;
  form: WorkspaceVariableFormState;
  keyEditable: boolean;
  mode: "create" | "edit" | "override";
  onChange: (next: WorkspaceVariableFormState) => void;
}) {
  return (
    <div className="space-y-5">
      <Field label="Variable Key" hint={keyEditable ? "Letters, digits, underscore, hyphen, or dot." : undefined}>
        <TextInput
          aria-label="Variable key"
          disabled={!keyEditable}
          value={form.key}
          onChange={(event) => onChange({ ...form, key: event.target.value })}
        />
      </Field>
      <Field label="Sensitive Data">
        <SensitiveDataCheckbox
          checked={form.isSensitive}
          disabled={disableSensitiveDataToggle}
          onChange={(checked) => onChange({
            ...form,
            isSensitive: checked,
            valueMode: "REPLACE",
          })}
        />
      </Field>
      <Field
        label="Variable Value"
        hint={form.isSensitive
          ? "Masked by default and requires verification to reveal."
          : mode === "override"
            ? "Save a local value in this workspace without mutating the inherited source."
            : "Value is stored exactly as entered; no data typing is applied."}
      >
        <TextArea
          aria-label="Variable value"
          className="min-h-32 font-mono"
          value={form.value}
          onChange={(event) => onChange({ ...form, value: event.target.value, valueMode: "REPLACE" })}
        />
      </Field>
      <Field label="Description" hint="Optional operator-facing note about how this variable is used.">
        <TextArea
          aria-label="Variable description"
          className="min-h-24"
          value={form.description}
          onChange={(event) => onChange({ ...form, description: event.target.value })}
        />
      </Field>
    </div>
  );
}
