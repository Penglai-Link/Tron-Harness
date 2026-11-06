import { useState } from "react";
import { TextInput } from "@/components/forms";

interface BudgetShareControlProps {
  value: number | null;
  label: string;
  placeholder: string;
  disabled: boolean;
  onChange: (basisPoints: number | null) => void;
}

export function BudgetShareControl({ value, label, placeholder, disabled, onChange }: BudgetShareControlProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const effective = value ?? 0;
  return (
    <div className="flex min-w-0 items-center gap-3">
      <input type="range" min={1} max={10_000} step={1} value={Math.max(1, Math.min(10_000, effective))} disabled={disabled}
        aria-label={label} aria-valuetext={`${effective / 100}%`}
        className="h-1.5 min-w-0 flex-1 cursor-pointer appearance-none rounded-full accent-[var(--color-accent)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] disabled:cursor-default disabled:opacity-50 [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[var(--color-accent)] [&::-moz-range-thumb]:size-3 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-[var(--color-accent)]"
        style={{ background: `linear-gradient(to right, var(--color-accent) ${effective / 100}%, var(--color-outline-variant) ${effective / 100}%)` }}
        onChange={event => onChange(Number(event.currentTarget.value))} />
      <div className="relative w-[4.75rem] shrink-0">
        <TextInput type="text" inputMode="decimal" value={editing ? draft : value == null ? "" : value / 100}
          disabled={disabled} aria-invalid={value != null && (!Number.isSafeInteger(value) || value < 1 || value > 10_000)} aria-label={label} placeholder={placeholder}
          className="h-8 border-transparent bg-transparent px-1 pr-5 text-right text-sm tabular-nums shadow-none hover:border-[var(--color-outline-variant)] focus:border-[var(--color-focus-ring)] [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
          onFocus={() => { setDraft(value == null ? "" : String(value / 100)); setEditing(true); }}
          onBlur={() => { setEditing(false); if (draft === "") onChange(null); }}
          onChange={event => {
            const raw = event.currentTarget.value;
            if (!/^\d*(?:\.\d*)?$/.test(raw)) return;
            setDraft(raw);
            const scaled = Number(raw) * 100;
            const parsed = Math.abs(scaled - Math.round(scaled)) < 1e-6 ? Math.round(scaled) : scaled;
            if (raw === "") onChange(null);
            else if (Number.isFinite(parsed)) onChange(parsed);
          }} />
        <span className="pointer-events-none absolute inset-y-0 right-1 flex items-center text-xs text-[var(--color-on-surface-variant)]">%</span>
      </div>
    </div>
  );
}
