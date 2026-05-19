import type {
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";

import { cn } from "@/shared/lib/utils";

interface FieldProps {
  children: ReactNode;
  error?: string;
  hint?: string;
  label: string;
}

export function Field({ children, error, hint, label }: FieldProps) {
  return (
    <label className="grid gap-1.5">
      <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</span>
      {children}
      {error ? <span className="text-[12px] text-[var(--color-error)]">{error}</span> : null}
      {hint && !error ? <span className="text-[12px] text-[var(--color-on-surface-variant)]">{hint}</span> : null}
    </label>
  );
}

export function TextInput({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={cn(
        "h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] outline-none transition-colors placeholder:text-[var(--color-outline)] focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
        className,
      )}
      {...props}
    />
  );
}

export function TextArea({ className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        "min-h-28 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm text-[var(--color-on-surface)] outline-none transition-colors placeholder:text-[var(--color-outline)] focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
        className,
      )}
      {...props}
    />
  );
}

export function SelectInput({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={cn(
        "h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] outline-none transition-colors focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
        className,
      )}
      {...props}
    />
  );
}

interface ToggleSwitchProps {
  checked: boolean;
  label: string;
  onCheckedChange: (checked: boolean) => void;
}

export function ToggleSwitch({ checked, label, onCheckedChange }: ToggleSwitchProps) {
  return (
    <button className="flex items-center gap-3 text-sm text-[var(--color-on-surface)]" type="button" aria-pressed={checked} onClick={() => onCheckedChange(!checked)}>
      <span
        className={cn(
          "relative inline-flex h-5 w-9 items-center rounded-full transition-colors",
          checked ? "bg-[var(--color-primary)]" : "bg-[var(--color-surface-container-highest)]",
        )}
      >
        <span className={cn("absolute size-4 rounded-full bg-[var(--color-elevated-surface-strong)] shadow transition-transform", checked ? "translate-x-[18px]" : "translate-x-0.5")} />
      </span>
      <span>{label}</span>
    </button>
  );
}
