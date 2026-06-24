import { SelectInput } from "@/shared/ui/Forms";

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
      onChange={(event) => onChange(event.target.value)}
    >
      {options.length ? options.map((option) => (
        <option disabled={option.disabled} key={option.value} value={option.value}>
          {option.label}
        </option>
      )) : (
        <option value="">
          {loading ? loadingLabel : emptyLabel}
        </option>
      )}
    </SelectInput>
  );
}
