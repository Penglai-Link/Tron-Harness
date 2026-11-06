import { Children, isValidElement, type ReactNode } from "react";

import {
  Field,
  FieldDescription,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";

interface TextFieldProps {
  description?: string;
  autoComplete?: string;
  disabled?: boolean;
  id: string;
  label: string;
  min?: number;
  onChange: (value: string) => void;
  placeholder?: string;
  readOnly?: boolean;
  required?: boolean;
  type?: "number" | "password" | "text";
  value: string;
}

export function ConnectionTextField({
  description,
  id,
  label,
  onChange,
  type,
  min,
  ...props
}: TextFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}{props.required ? <span aria-hidden="true" className="text-destructive"> *</span> : null}</FieldLabel>
      <Input id={id} type={type === "number" ? "text" : type} inputMode={type === "number" ? "numeric" : undefined}
        pattern={type === "number" ? "[0-9]*" : undefined} onChange={(event) => {
          const raw = event.target.value;
          if (type === "number" && !/^\d*$/.test(raw)) return;
          event.currentTarget.setCustomValidity(type === "number" && raw !== "" && min !== undefined && Number(raw) < min ? `Minimum: ${min}` : "");
          onChange(raw);
        }} {...props} />
      {description ? <FieldDescription>{description}</FieldDescription> : null}
    </Field>
  );
}

interface TextareaFieldProps {
  required?: boolean;
  description?: string;
  id: string;
  label: string;
  onChange: (value: string) => void;
  placeholder?: string;
  value: string;
}

export function ConnectionTextareaField({
  description,
  id,
  label,
  onChange,
  ...props
}: TextareaFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}{props.required ? <span aria-hidden="true" className="text-destructive"> *</span> : null}</FieldLabel>
      <Textarea id={id} onChange={(event) => onChange(event.target.value)} autoComplete="off" spellCheck={false} {...props} />
      {description ? <FieldDescription>{description}</FieldDescription> : null}
    </Field>
  );
}

interface SelectFieldProps {
  required?: boolean;
  children: ReactNode;
  description?: string;
  id: string;
  label: string;
  onValueChange: (value: string) => void;
  value: string;
}

export function ConnectionSelectField({
  required,
  children,
  description,
  id,
  label,
  onValueChange,
  value,
}: SelectFieldProps) {
  const selectedItem = Children.toArray(children).find((child) => isValidElement<{ value: string }>(child) && child.props.value === value);
  const selectedLabel = isValidElement<{ children: ReactNode }>(selectedItem) ? selectedItem.props.children : undefined;
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}{required ? <span aria-hidden="true" className="text-destructive"> *</span> : null}</FieldLabel>
      <Select required={required} onValueChange={(next) => next && onValueChange(next)} value={value}>
        <SelectTrigger id={id} aria-required={required}>
          <SelectValue>{selectedLabel}</SelectValue>
        </SelectTrigger>
        <SelectPopup>{children}</SelectPopup>
      </Select>
      {description ? <FieldDescription>{description}</FieldDescription> : null}
    </Field>
  );
}

export { SelectItem as ConnectionSelectItem };
