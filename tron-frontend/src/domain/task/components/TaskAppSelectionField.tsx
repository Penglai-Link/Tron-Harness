import { cn } from "@/shared/lib/utils";

import type { AppOption } from "@/domain/app/model";

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
    <span className="inline-flex items-center gap-2 rounded-full border border-[var(--color-warning-container)] bg-[var(--color-warning-container)] px-3 py-1 text-sm text-[var(--color-on-warning-container)]">
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
  const availableCodes = new Set(availableApps.map((app) => app.appCode));
  const selectedKnownCodes = selectedAppCodes.filter((code) => availableCodes.has(code));

  return (
    <div className="space-y-3">
      {availableApps.length ? (
        <div className="flex flex-wrap gap-2" aria-label="Available apps">
          {availableApps.map((app) => {
            const active = selectedKnownCodes.includes(app.appCode);
            return (
              <button
                className={cn(
                  "min-w-[12rem] rounded-2xl border px-3 py-2 text-left text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
                  active
                    ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]"
                    : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]",
                )}
                disabled={disabled || !onToggle}
                type="button"
                key={app.appCode}
                aria-pressed={active}
                onClick={() => onToggle?.(app.appCode)}
              >
                <span className="block font-medium">{app.name}</span>
                <span className={cn("mt-1 block font-mono text-[11px]", active ? "text-[var(--color-on-primary)]/85" : "text-[var(--color-on-surface-variant)]")}>
                  {app.appCode} · {app.runtimeRole}
                </span>
              </button>
            );
          })}
        </div>
      ) : loading ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">Loading app options...</p>
      ) : null}
      {legacyAppCodes.length ? (
        <div className="space-y-2">
          <p className="text-[12px] text-[var(--color-on-surface-variant)]">
            Legacy app codes are not registered in the current workspace. They stay visible until you explicitly remove or replace them.
          </p>
          <div className="flex flex-wrap gap-2" aria-label="Legacy app codes">
            {legacyAppCodes.map((code) => (
              <LegacyChip key={code} code={code} disabled={disabled} onRemove={onRemoveLegacy} />
            ))}
          </div>
        </div>
      ) : null}
      {!availableApps.length && !legacyAppCodes.length && !loading ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">{emptyMessage}</p>
      ) : null}
    </div>
  );
}
