import type { AppOption } from "@/domain/app/model";
import { MultiSelectDropdown } from "@/components/multi-select-dropdown";
import { useTranslation } from "react-i18next";

function LegacyChip({
  code,
  disabled,
  onRemove,
}: {
  code: string;
  disabled: boolean;
  onRemove?: (code: string) => void;
}) {
  return (
    <span className="inline-flex items-center gap-[12px] rounded-full border border-[var(--color-warning-container)] bg-[var(--color-warning-container)] px-3 py-1 text-sm text-[var(--color-on-warning-container)]">
      <span>{code}</span>
      {onRemove ? (
        <button
          className="rounded-full px-1 text-[12px] font-semibold hover:bg-black/10 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={disabled}
          type="button"
          aria-label={`Remove legacy app code ${code}`}
          onClick={() => onRemove(code)}
        >
          x
        </button>
      ) : null}
    </span>
  );
}

export function TaskAppSelectionField({
  availableApps,
  disabled = false,
  emptyMessage,
  legacyAppCodes = [],
  loading = false,
  onRemoveLegacy,
  onToggle,
  selectedAppCodes,
}: {
  availableApps: AppOption[];
  disabled?: boolean;
  emptyMessage: string;
  legacyAppCodes?: string[];
  loading?: boolean;
  onRemoveLegacy?: (code: string) => void;
  onToggle?: (code: string) => void;
  selectedAppCodes: string[];
}) {
  const { t } = useTranslation(["pages"]);
  const text = (key: string, fallback: string, options?: Record<string, unknown>) => t(`pages:tasks.apps.${key}`, { defaultValue: fallback, ...options });
  const availableCodes = new Set(availableApps.map((app) => app.appCode));
  const selectedKnownCodes = selectedAppCodes.filter((code) => availableCodes.has(code));
  const appOptions = availableApps.map((app) => ({
    description: `${app.appCode} · ${app.runtimeRole}`,
    label: app.name,
    value: app.appCode,
  }));

  return (
    <div className="space-y-3 w-full">
      <MultiSelectDropdown
        ariaLabel={text("ariaLabel", "Task apps")}
        disabled={disabled}
        emptyMessage={emptyMessage}
        loading={loading}
        loadingMessage={text("loading", "Loading app options...")}
        options={appOptions}
        placeholder={text("placeholder", "Select apps")}
        selectedValues={selectedKnownCodes}
        onToggle={onToggle}
      />
      {legacyAppCodes.length ? (
        <div className="space-y-2">
          <p className="text-[12px] text-[var(--color-on-surface-variant)]">
            {text("legacyHint", "Legacy app codes are not registered in the current workspace. They stay visible until you explicitly remove or replace them.")}
          </p>
          <div className="flex flex-wrap gap-[12px]" aria-label={text("legacyLabel", "Legacy app codes")}>
            {legacyAppCodes.map((code) => (
              <LegacyChip key={code} code={code} disabled={disabled} onRemove={onRemoveLegacy} />
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}
