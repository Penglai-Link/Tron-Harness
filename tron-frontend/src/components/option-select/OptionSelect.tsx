import { SelectItem } from "@/components/ui/select";
import { SelectInput } from "@/components/forms";

export interface OptionSelectItem {
  disabled?: boolean;
  label: string;
  value: string;
}

export function OptionSelect({
  ariaLabel,
  "aria-describedby": ariaDescribedBy,
  disabled = false,
  emptyLabel = "No options",
  id,
  loading = false,
  loadingLabel = "Loading options...",
  onChange,
  options,
  value,
}: {
  "aria-describedby"?: string;
  ariaLabel: string;
  disabled?: boolean;
  emptyLabel?: string;
  id?: string;
  loading?: boolean;
  loadingLabel?: string;
  onChange: (value: string) => void;
  options: OptionSelectItem[];
  value: string;
}) {
  return (
    <SelectInput
      aria-label={ariaLabel}
      aria-describedby={ariaDescribedBy}
      disabled={disabled}
      id={id}
      value={value}
      onValueChange={(event) => onChange(event!)}
    >
      {options.length ? options.map((option) => (
        <SelectItem disabled={option.disabled} key={option.value} value={option.value}>
          {option.label}
        </SelectItem>
      )) : (
        <SelectItem value="">
          {loading ? loadingLabel : emptyLabel}
        </SelectItem>
      )}
    </SelectInput>
  );
}
