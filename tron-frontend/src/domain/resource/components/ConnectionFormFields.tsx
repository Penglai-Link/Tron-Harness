import type { ReactNode } from "react";

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
  ...props
}: TextFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input id={id} onChange={(event) => onChange(event.target.value)} {...props} />
      {description ? <FieldDescription>{description}</FieldDescription> : null}
    </Field>
  );
}

interface TextareaFieldProps {
  id: string;
  label: string;
  onChange: (value: string) => void;
  placeholder?: string;
  value: string;
}

export function ConnectionTextareaField({
  id,
  label,
  onChange,
  ...props
}: TextareaFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Textarea id={id} onChange={(event) => onChange(event.target.value)} {...props} />
    </Field>
  );
}

interface SelectFieldProps {
  children: ReactNode;
  description?: string;
  id: string;
  label: string;
  onValueChange: (value: string) => void;
  value: string;
}

export function ConnectionSelectField({
  children,
  description,
  id,
  label,
  onValueChange,
  value,
}: SelectFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Select onValueChange={(next) => next && onValueChange(next)} value={value}>
        <SelectTrigger id={id}>
          <SelectValue />
        </SelectTrigger>
        <SelectPopup>{children}</SelectPopup>
      </Select>
      {description ? <FieldDescription>{description}</FieldDescription> : null}
    </Field>
  );
}

export { SelectItem as ConnectionSelectItem };
