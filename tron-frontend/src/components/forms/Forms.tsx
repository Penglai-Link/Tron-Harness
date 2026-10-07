import type {
  ForwardedRef,
  InputHTMLAttributes,
  ReactNode,
  TextareaHTMLAttributes,
} from "react";
import { Children, forwardRef, isValidElement, useMemo } from "react";

import { cn } from "@/shared/lib/utils";
import { InfoTooltip } from "@/components/info-tooltip";
import { Select, SelectPopup, SelectPrimitive, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Field as FieldPrimitive, FieldLabel, FieldError } from "@/components/ui/field";

interface FieldProps {
  className?: string;
  children: ReactNode;
  error?: string;
  hint?: ReactNode;
  hintAriaLabel?: string;
  label: ReactNode;
  required?: boolean;
}

const disabledControlClasses =
  "disabled:cursor-not-allowed disabled:border-[var(--tron-input-disabled-border,var(--color-outline-variant))] disabled:bg-[var(--tron-input-disabled-bg,var(--color-surface-container))] disabled:text-[var(--tron-input-disabled-fg,var(--color-on-surface-variant))] disabled:opacity-[var(--tron-input-disabled-opacity,1)] disabled:placeholder:text-[var(--tron-input-placeholder,var(--color-outline))] disabled:focus:border-[var(--color-outline-variant)] disabled:focus:ring-0";

export function Field({ children, className, error, hint, hintAriaLabel = "打开字段说明", label, required = false }: FieldProps) {
  return (
    <FieldPrimitive className={className}>
      <div className="flex items-center gap-1">
        <FieldLabel>
          {label}
          {required && <span className="text-destructive-foreground" aria-hidden="true">*</span>}
        </FieldLabel>
        {hint && <InfoTooltip ariaLabel={hintAriaLabel}>{hint}</InfoTooltip>}
      </div>
      {children}
      {error && <FieldError match>{error}</FieldError>}
    </FieldPrimitive>
  );
}

export const TextInput = forwardRef(function TextInput(
  { className, size, ...props }: InputHTMLAttributes<HTMLInputElement>,
  ref: ForwardedRef<HTMLInputElement>,
) {
  return (
    <Input
      size={size}
      ref={ref}
      className={cn(
        "h-[var(--tron-input-height,2.25rem)] sm:h-[var(--tron-input-height,2.125rem)]",
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

export function SelectInput<Value, Multiple extends boolean | undefined = false>({
  "aria-describedby": ariaDescribedBy,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledBy,
  children,
  className,
  displayValue,
  id,
  placeholder,
  popupPositionerClassName,
  size,
  ...props
}: SelectPrimitive.Root.Props<Value, Multiple> & {
  "aria-describedby"?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  className?: string;
  displayValue?: SelectPrimitive.Value.Props["children"];
  id?: string;
  placeholder?: ReactNode;
  popupPositionerClassName?: string;
  size?: 'sm' | 'lg';
}) {
  const items = useMemo(() => {
    return Children.toArray(children).map((child) => {
      if (isValidElement(child)) {
        return {
          value: (child.props as SelectPrimitive.Item.Props).value,
          label: (child.props as SelectPrimitive.Item.Props).children,
        };
      }
      return [];
    }).filter(Boolean) as ReadonlyArray<{
      label: ReactNode;
      value: any;
    }>;
  }, [children]);

  return (
    <>
      <Select {...props} items={items}>
        <SelectTrigger
          aria-describedby={ariaDescribedBy}
          aria-label={ariaLabel}
          aria-labelledby={ariaLabelledBy}
          id={id}
          size={size || 'lg'}
          className={cn(
            disabledControlClasses,
            className,
          )}
        >
          <SelectValue placeholder={placeholder}>
            {displayValue}
          </SelectValue>
        </SelectTrigger>
        <SelectPopup alignItemWithTrigger={false} positionerClassName={popupPositionerClassName}>
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
        <Switch checked={checked} disabled={disabled} onCheckedChange={() => {
          if (disabled) return;
          onCheckedChange(!checked);
        }} />
        {label ? <span>{label}</span> : null}
      </Label>
    </>
  );
}
