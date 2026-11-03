import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Combobox, ComboboxInput, ComboboxItem, ComboboxList, ComboboxPopup, ComboboxTrigger } from "@/components/ui/combobox";

const windowPresets = [
  { label: "500K", tokens: 500_000 },
  { label: "1M", tokens: 1_000_000 },
  { label: "2M", tokens: 2_000_000 },
];

export function ContextWindowInput({ value, onChange }: { value: number; onChange: (tokens: number) => void }) {
  const { t, i18n } = useTranslation("pages");
  const [draft, setDraft] = useState<string | null>(null);
  const number = new Intl.NumberFormat(i18n.language);

  return <Combobox items={windowPresets} value={windowPresets.find(preset => preset.tokens === value) ?? null}
    inputValue={draft ?? String(value)} itemToStringLabel={preset => String(preset.tokens)}
    filter={() => true}
    onInputValueChange={(text, details) => {
      if (details.reason !== "input-change") return;
      setDraft(text);
      const digits = text.replace(/[\s,]/g, "");
      const tokens = /^\d+$/.test(digits) ? Number(digits) : NaN;
      if (Number.isSafeInteger(tokens) && tokens >= 1000 && tokens <= 100_000_000) onChange(tokens);
    }}
    onValueChange={preset => {
      if (!preset) return;
      setDraft(null);
      onChange(preset.tokens);
    }}>
    <div className="relative w-40">
      <ComboboxInput type="text" inputMode="numeric" autoComplete="off" showTrigger={false}
        className="h-8 pr-8 text-right tabular-nums" aria-label={t("contextLayout.previewWindow")}
        onBlur={() => setDraft(null)} />
      <ComboboxTrigger aria-label={t("contextLayout.previewWindow")} className="absolute inset-y-0 right-0 flex w-8 items-center justify-center rounded-r text-[var(--color-on-surface-variant)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]">
        <ChevronDown size={14} aria-hidden="true" />
      </ComboboxTrigger>
    </div>
    <ComboboxPopup className="min-w-56">
      <ComboboxList>{(preset: typeof windowPresets[number]) => <ComboboxItem key={preset.tokens} value={preset}>
        <div className="flex items-center justify-between gap-6 tabular-nums">
          <span className="font-medium">{preset.label}</span>
          <span className="text-xs text-[var(--color-on-surface-variant)]">{number.format(preset.tokens)}</span>
        </div>
      </ComboboxItem>}</ComboboxList>
    </ComboboxPopup>
  </Combobox>;
}
