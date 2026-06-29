import type {
  ForwardedRef,
  InputHTMLAttributes,
  ReactElement,
  ReactNode,
  TextareaHTMLAttributes,
} from "react";
import { cloneElement, forwardRef, isValidElement, useId } from "react";

import { cn } from "@/shared/lib/utils";
import { InfoTooltip } from "@/shared/ui/InfoTooltip";
import { Select, SelectPopup, SelectPrimitive, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

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
    <Input
      size={props.size || 'lg'}
      ref={ref}
      className={cn(
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
    <Textarea
      ref={ref}
      className={cn(
        disabledControlClasses,
        className,
      )}
      {...props}
    />
  );
});



export function SelectInput<Value, Multiple extends boolean | undefined = false>({ className, children, ...props }: SelectPrimitive.Root.Props<Value, Multiple> & { className?: string; size?: 'sm' | 'lg' }) {
  return (
    <>
      <Select {...props}>
        <SelectTrigger size={props.size || 'lg'} className={cn(
          disabledControlClasses,
          className,
        )}>
          <SelectValue />
        </SelectTrigger>
        <SelectPopup alignItemWithTrigger={false}>
          {children}
        </SelectPopup>
      </Select>
    </>
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
    <>
      <Label className="text-[var(--color-on-surface)]" title={'12'} aria-label={ariaLabel ?? label}>
        <Switch disabled={disabled} onCheckedChange={() => {
          if (disabled) return;
          onCheckedChange(!checked);
        }} />
        {label ? <span>{label}</span> : null}
      </Label>
    </>
  );
}
