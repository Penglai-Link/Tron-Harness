import type {
  ForwardedRef,
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";
import { forwardRef } from "react";

import { cn } from "@/shared/lib/utils";

interface FieldProps {
  children: ReactNode;
  error?: string;
  hint?: string;
  label: ReactNode;
}

const disabledControlClasses =
  "disabled:cursor-not-allowed disabled:border-[var(--color-outline-variant)] disabled:bg-[var(--color-surface-container)] disabled:text-[var(--color-on-surface-variant)] disabled:opacity-100 disabled:placeholder:text-[var(--color-outline)] disabled:focus:border-[var(--color-outline-variant)] disabled:focus:ring-0";

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

export const TextInput = forwardRef(function TextInput(
  { className, ...props }: InputHTMLAttributes<HTMLInputElement>,
  ref: ForwardedRef<HTMLInputElement>,
) {
  return (
    <input
      ref={ref}
      className={cn(
        "h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] outline-none transition-colors placeholder:text-[var(--color-outline)] focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
        disabledControlClasses,
        className,
      )}
      {...props}
    />
  );
});

export const TextArea = forwardRef(function TextArea(
  { className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>,
  ref: ForwardedRef<HTMLTextAreaElement>,
) {
  return (
    <textarea
      ref={ref}
      className={cn(
        "min-h-28 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm text-[var(--color-on-surface)] outline-none transition-colors placeholder:text-[var(--color-outline)] focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
        disabledControlClasses,
        className,
      )}
      {...props}
    />
  );
});

export function SelectInput({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={cn(
        "h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] outline-none transition-colors focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
        disabledControlClasses,
        className,
      )}
      {...props}
    />
  );
}

interface ToggleSwitchProps {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onCheckedChange: (checked: boolean) => void;
}

export function ToggleSwitch({ checked, disabled = false, label, onCheckedChange }: ToggleSwitchProps) {
  return (
    <button
      className={cn(
        "flex items-center gap-3 text-sm text-[var(--color-on-surface)]",
        disabled ? "cursor-not-allowed opacity-60" : undefined,
      )}
      type="button"
      aria-disabled={disabled}
      aria-pressed={checked}
      disabled={disabled}
      onClick={() => {
        if (disabled) return;
        onCheckedChange(!checked);
      }}
    >
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
