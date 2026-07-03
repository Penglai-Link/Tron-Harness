import type {
  ForwardedRef,
  InputHTMLAttributes,
  ReactElement,
  ReactNode,
  TextareaHTMLAttributes
} from "react";
import { Children } from "react"
import { cloneElement, forwardRef, isValidElement, useId, useMemo } from "react";

import { cn } from "@/shared/lib/utils";
import { InfoTooltip } from "@/shared/ui/InfoTooltip";
import { Select, SelectContent, SelectItem, SelectPopup, SelectPrimitive, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Field as FieldPrimitive, FieldLabel, FieldError } from "@/components/ui/field";
import { SelectItemProps } from "@base-ui/react/select";


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
  return (
    <FieldPrimitive>
      <FieldLabel>
        {label}
        {
          hint && <InfoTooltip ariaLabel="打开字段说明">
            {hint}
          </InfoTooltip>
        }
      </FieldLabel>
      {children}
      {error && <FieldError match>{error}</FieldError>}
    </FieldPrimitive>
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




export function SelectInput<Value, Multiple extends boolean | undefined = false>({
  "aria-describedby": ariaDescribedBy,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledBy,
  children,
  className,
  id,
  placeholder,
  size,
  ...props
}: SelectPrimitive.Root.Props<Value, Multiple> & {
  "aria-describedby"?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  className?: string;
  id?: string;
  placeholder?: ReactNode;
  size?: 'sm' | 'lg';
}) {

  const items = useMemo(() => {
    return Children.toArray(children).map((child) => {
      if (isValidElement(child)) {
        return {
          value: (child.props as SelectPrimitive.Item.Props).value,
          label: (child.props as SelectPrimitive.Item.Props).children,
        }
      }
      return [];
    }).filter(Boolean) as ReadonlyArray<{
      label: React.ReactNode;
      value: any;
    }>;
  }, [children])

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
          )}>
          <SelectValue placeholder={placeholder} />
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
        <Switch checked={checked} disabled={disabled} onCheckedChange={() => {
          if (disabled) return;
          onCheckedChange(!checked);
        }} />
        {label ? <span>{label}</span> : null}
      </Label>
    </>
  );
}
