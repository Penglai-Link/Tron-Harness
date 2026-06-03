import {
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  Eye,
  EyeOff,
  ShieldCheck,
} from "lucide-react";

import {
  Field,
  TextArea,
  TextInput,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import type {
  PlatformVariableRecord,
  VariableValueMode,
} from "@/domain/variable/model";

export const sensitiveMask = "********";

export interface PlatformVariableFormState {
  description: string;
  isSensitive: boolean;
  key: string;
  value: string;
  valueMode: VariableValueMode;
}

export function emptyPlatformVariableForm(): PlatformVariableFormState {
  return {
    description: "",
    isSensitive: false,
    key: "",
    value: "",
    valueMode: "REPLACE",
  };
}

export function platformVariableFormFromRecord(record: PlatformVariableRecord): PlatformVariableFormState {
  const isSensitive = isSensitiveRecord(record);
  return {
    description: record.description ?? "",
    isSensitive,
    key: record.key,
    value: isSensitive ? "" : record.value,
    valueMode: isSensitive ? "PRESERVE" : "REPLACE",
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
    isSensitiveRecord(record) ? "" : record.value,
    record.description ?? "",
  ].join(" ").toLowerCase().includes(normalized);
}

export function getSensitiveState(isSensitive: boolean, revealedValue: string | undefined) {
  if (!isSensitive) return "plain";
  return revealedValue ? "revealed" : "masked";
}

export function getSensitiveDisplayValue(record: SensitiveDisplayRecord, revealedValue?: string) {
  if (!isSensitiveRecord(record)) return record.value;
  return revealedValue || sensitiveMask;
}

interface SensitiveDisplayRecord {
  isSensitive?: boolean;
  value: string;
  valueMasked?: boolean;
}

export function isSensitiveRecord(record: SensitiveDisplayRecord) {
  return Boolean(record.isSensitive || record.valueMasked);
}

export function ReadValue({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children || "None"}
    </div>
  );
}

export function SensitiveValueCode({
  children,
  className = "",
  testId,
  state,
}: {
  children: ReactNode;
  className?: string;
  state: "masked" | "plain" | "revealed";
  testId?: string;
}) {
  return (
    <code
      className={`whitespace-pre-wrap break-words text-sm leading-6 text-[var(--color-on-surface)] ${className}`}
      data-sensitive-state={state}
      data-testid={testId}
    >
      {children}
    </code>
  );
}

export function SensitiveRevealAction({
  isSensitive,
  onHide,
  onReveal,
  revealed,
  variableKey,
}: {
  isSensitive: boolean;
  onHide: () => void;
  onReveal: () => void;
  revealed: boolean;
  variableKey: string;
}) {
  if (!isSensitive) return null;

  return (
    <Button
      type="button"
      size="sm"
      variant="secondary"
      aria-label={`${revealed ? "Hide" : "Reveal"} ${variableKey}`}
      onClick={revealed ? onHide : onReveal}
    >
      {revealed ? <EyeOff size={14} aria-hidden="true" /> : <Eye size={14} aria-hidden="true" />}
      {revealed ? "Hide" : "Reveal"}
    </Button>
  );
}

export function StepUpRevealDialog({
  error,
  onCancel,
  onSubmit,
  open,
  pending,
  variableKey,
}: {
  error?: string;
  onCancel: () => void;
  onSubmit: (password: string) => void;
  open: boolean;
  pending?: boolean;
  variableKey: string;
}) {
  const [password, setPassword] = useState("");

  useEffect(() => {
    setPassword("");
  }, [open, variableKey]);

  useEffect(() => {
    if (!open) return undefined;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !pending) {
        onCancel();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onCancel, open, pending]);

  if (!open) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[110] flex items-center justify-center bg-black/45 p-4 backdrop-blur-sm"
      onMouseDown={(event) => {
        if (event.target !== event.currentTarget || pending) return;
        onCancel();
      }}
    >
      <div
        className="w-[min(92vw,32rem)] rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
        role="dialog"
        aria-modal="true"
        aria-label={`Reveal ${variableKey}`}
      >
        <div className="flex items-start gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
          <div className="mt-0.5 rounded-full bg-[var(--color-primary-container)] p-2 text-[var(--color-on-primary-container)]">
            <ShieldCheck size={18} aria-hidden="true" />
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Reveal Sensitive Value</h2>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
              Verify your current password to reveal this value temporarily.
            </p>
          </div>
        </div>
        <form
          className="space-y-4 px-5 py-4"
          onSubmit={(event) => {
            event.preventDefault();
            onSubmit(password);
          }}
        >
          <Field label="Step-up Password" error={error}>
            <TextInput
              aria-label="Step-up password"
              autoFocus
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </Field>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" disabled={pending} onClick={onCancel}>
              Cancel
            </Button>
            <Button type="submit" disabled={pending || !password.trim()} aria-busy={pending}>
              <Eye size={16} aria-hidden="true" />
              {pending ? "Revealing..." : "Reveal Value"}
            </Button>
          </div>
        </form>
      </div>
    </div>,
    document.body,
  );
}

function SensitiveToggleButton({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <Button
      type="button"
      variant={checked ? "primary" : "secondary"}
      aria-label="Sensitive value"
      aria-pressed={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <ShieldCheck size={16} aria-hidden="true" />
      Sensitive value
    </Button>
  );
}

function ValueModeButton({
  active,
  children,
  onClick,
}: {
  active: boolean;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      variant={active ? "primary" : "secondary"}
      aria-label={typeof children === "string" ? children : undefined}
      aria-pressed={active}
      onClick={onClick}
    >
      {children}
    </Button>
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
      <Field label="Sensitivity">
        <SensitiveToggleButton
          checked={form.isSensitive}
          onChange={(checked) => onChange({
            ...form,
            isSensitive: checked,
            value: checked ? "" : form.value,
            valueMode: "REPLACE",
          })}
        />
      </Field>
      {form.isSensitive && !keyEditable ? (
        <Field label="Sensitive Value Mode" hint="Preserve keeps the stored encrypted value; replace submits a new value.">
          <div className="flex flex-wrap gap-2">
            <ValueModeButton
              active={form.valueMode === "PRESERVE"}
              onClick={() => onChange({ ...form, value: "", valueMode: "PRESERVE" })}
            >
              Keep existing sensitive value
            </ValueModeButton>
            <ValueModeButton
              active={form.valueMode === "REPLACE"}
              onClick={() => onChange({ ...form, valueMode: "REPLACE" })}
            >
              Replace sensitive value
            </ValueModeButton>
          </div>
        </Field>
      ) : null}
      <Field
        label="Variable Value"
        hint={form.isSensitive
          ? form.valueMode === "PRESERVE"
            ? "The current sensitive value stays encrypted and is not loaded into the form."
            : "The replacement is sent once and returns to a mask after save."
          : "This value becomes the inherited default for all workspaces."}
      >
        <TextArea
          aria-label="Platform variable value"
          className="min-h-44 font-mono"
          disabled={form.isSensitive && form.valueMode === "PRESERVE"}
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
