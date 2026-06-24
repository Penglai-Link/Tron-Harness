import type {
  ForwardedRef,
  InputHTMLAttributes,
  ReactElement,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";
import { cloneElement, forwardRef, isValidElement, useId } from "react";

import { cn } from "@/shared/lib/utils";
import { InfoTooltip } from "@/shared/ui/InfoTooltip";

interface FieldProps {
  children: ReactNode;
  error?: string;
  hint?: string;
  label: ReactNode;
}

interface FieldChildProps {
  "aria-describedby"?: string;
  id?: string;
}

const disabledControlClasses =
  "disabled:cursor-not-allowed disabled:border-[var(--color-outline-variant)] disabled:bg-[var(--color-surface-container)] disabled:text-[var(--color-on-surface-variant)] disabled:opacity-100 disabled:placeholder:text-[var(--color-outline)] disabled:focus:border-[var(--color-outline-variant)] disabled:focus:ring-0";

function mergeAriaReferences(...references: Array<string | undefined>) {
  const normalized = references.map((value) => value?.trim()).filter(Boolean);
  return normalized.length > 0 ? normalized.join(" ") : undefined;
}

export function Field({ children, error, hint, label }: FieldProps) {
  const fieldId = useId().replace(/:/g, "");
  const tooltipLabel = typeof label === "string" ? label : "Field";
  const hintId = hint ? `field-hint-${fieldId}` : undefined;
  const errorId = error ? `field-error-${fieldId}` : undefined;
  let controlId: string | undefined;
  let content = children;

  if (isValidElement<FieldChildProps>(children)) {
    const child = children as ReactElement<FieldChildProps>;
    controlId = child.props.id ?? `field-control-${fieldId}`;
    content = cloneElement(child, {
      id: controlId,
      "aria-describedby": mergeAriaReferences(child.props["aria-describedby"], hintId, errorId),
    });
  }

  return (
    <div className="grid min-w-0 gap-1.5">
      <div className="flex min-w-0 flex-wrap items-center gap-1.5">
        {controlId ? (
          <label className="min-w-0 break-words text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]" htmlFor={controlId}>
            {label}
          </label>
        ) : (
          <span className="min-w-0 break-words text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</span>
        )}
        {hint ? (
          <InfoTooltip ariaLabel="打开字段说明">
            <p id={hintId}>{hint}</p>
          </InfoTooltip>
        ) : null}
      </div>
      {content}
      {error ? <span className="text-[12px] text-[var(--color-error)]" id={errorId}>{error}</span> : null}
    </div>
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
        "h-10 min-w-0 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] outline-none transition-colors placeholder:text-[var(--color-outline)] focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
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
        "min-h-28 min-w-0 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm text-[var(--color-on-surface)] outline-none transition-colors placeholder:text-[var(--color-outline)] focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
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
        "h-10 min-w-0 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] outline-none transition-colors focus:border-[var(--color-primary)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20",
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
  ariaLabel?: string;
  label?: string;
  onCheckedChange: (checked: boolean) => void;
}

export function ToggleSwitch({ checked, disabled = false, ariaLabel, label, onCheckedChange }: ToggleSwitchProps) {
  return (
    <button
      className={cn(
        "flex items-center gap-3 text-sm text-[var(--color-on-surface)]",
        disabled ? "cursor-not-allowed opacity-60" : undefined,
      )}
      type="button"
      aria-label={ariaLabel ?? label}
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
      {label ? <span>{label}</span> : null}
    </button>
  );
}
