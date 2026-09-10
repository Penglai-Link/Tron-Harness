import {
  useEffect,
  useState,
  type MouseEvent,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import { createPortal } from "react-dom";
import {
  Eye,
  EyeOff,
  ShieldCheck,
} from "lucide-react";

import { Field, TextArea, TextInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/shared/lib/utils";

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
  const { t } = useTranslation(["pages"]);
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children || t("variablesCommon.none", "None")}
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

export function SensitiveRevealIconButton({
  onClick,
  revealed,
  variableKey,
}: {
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  revealed: boolean;
  variableKey: string;
}) {
  const { t } = useTranslation(["pages"]);
  const label = t(`variablesCommon.${revealed ? "hide" : "reveal"}`, { defaultValue: revealed ? "Hide" : "Reveal" });
  return (
    <Button
      type="button"
      size="icon"
      variant="ghost"
      aria-label={`${label} ${variableKey}`}
      title={label}
      onClick={onClick}
    >
      {revealed ? <EyeOff size={16} aria-hidden="true" /> : <Eye size={16} aria-hidden="true" />}
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
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string) => t(`variablesCommon.${key}`, { defaultValue });
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
        aria-label={`${pageT("reveal", "Reveal")} ${variableKey}`}
      >
        <div className="flex items-start gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
          <div className="mt-0.5 rounded-full bg-[var(--color-primary-container)] p-2 text-[var(--color-on-primary-container)]">
            <ShieldCheck size={18} aria-hidden="true" />
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("dialog.title", "Reveal Sensitive Value")}</h2>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
              {pageT("dialog.description", "Verify your current password to reveal this value temporarily.")}
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
          <Field label={pageT("dialog.password", "Step-up Password")} error={error}>
            <TextInput
              aria-label={pageT("dialog.password", "Step-up password")}
              autoFocus
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </Field>
          <div className="flex justify-end gap-[12px]">
            <Button type="button" variant="ghost" disabled={pending} onClick={onCancel}>
              {pageT("dialog.cancel", "Cancel")}
            </Button>
            <Button type="submit" disabled={pending || !password.trim()} aria-busy={pending}>
              <Eye size={16} aria-hidden="true" />
              {pending ? pageT("dialog.revealing", "Revealing...") : pageT("dialog.revealValue", "Reveal Value")}
            </Button>
          </div>
        </form>
      </div>
    </div>,
    document.body,
  );
}

export function SensitiveDataCheckbox({
  checked,
  disabled,
  label = "Sensitive Data",
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  label?: string;
  onChange: (checked: boolean) => void;
}) {
  const { t } = useTranslation(["pages"]);
  const resolvedLabel = label === "Sensitive Data" ? t("variablesCommon.sensitiveData", "Sensitive Data") : label;
  return (
    <label
      className={cn(
        "inline-flex min-h-8 w-fit items-center gap-[12px] text-sm text-[var(--color-on-surface)]",
        disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer",
      )}
    >
      <input
        type="checkbox"
        aria-label={resolvedLabel}
        checked={checked}
        className="size-4 accent-[var(--color-primary)]"
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span>{resolvedLabel}</span>
    </label>
  );
}

export function SensitiveDataBadge({
  sensitive,
}: {
  sensitive: boolean;
}) {
  const { t } = useTranslation(["pages"]);
  return (
    <span className="inline-flex w-fit max-w-max">
      <Badge
        tone={sensitive ? "warning" : "info"}
      >
        {sensitive ? t("variablesCommon.sensitive", "Sensitive") : t("variablesCommon.plain", "Plain")}
      </Badge>
    </span>
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
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string) => t(`variablesCommon.${key}`, { defaultValue });
  return (
    <div className="space-y-5">
      <Field label={pageT("form.key", "Variable Key")} hint={keyEditable ? pageT("form.keyHint", "Letters, digits, underscore, hyphen, or dot.") : undefined}>
        <TextInput
          aria-label={pageT("form.keyAria", "Platform variable key")}
          disabled={!keyEditable}
          value={form.key}
          onChange={(event) => onChange({ ...form, key: event.target.value })}
        />
      </Field>
      <Field label={pageT("sensitiveData", "Sensitive Data")}>
        <SensitiveDataCheckbox
          checked={form.isSensitive}
          onChange={(checked) => onChange({
            ...form,
            isSensitive: checked,
            valueMode: "REPLACE",
          })}
        />
      </Field>
      <Field
        label={pageT("form.value", "Variable Value")}
        hint={form.isSensitive
          ? pageT("form.maskedHint", "Masked by default and requires verification to reveal.")
          : pageT("form.inheritedHint", "This value becomes the inherited default for all workspaces.")}
      >
        <TextArea
          aria-label={pageT("form.valueAria", "Platform variable value")}
          className="min-h-44 font-mono"
          value={form.value}
          onChange={(event) => onChange({ ...form, value: event.target.value, valueMode: "REPLACE" })}
        />
      </Field>
      <Field label={pageT("form.description", "Description")} hint={pageT("form.descriptionHint", "Optional operator-facing note about how this platform variable is used.")}>
        <TextArea
          aria-label={pageT("form.descriptionAria", "Platform variable description")}
          className="min-h-24"
          value={form.description}
          onChange={(event) => onChange({ ...form, description: event.target.value })}
        />
      </Field>
    </div>
  );
}
