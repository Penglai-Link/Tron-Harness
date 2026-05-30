import { cn } from "@/shared/lib/utils";

interface AgentPolicySwitchProps {
  checked: boolean;
  disabled: boolean;
  label: string;
  loading: boolean;
  onToggle: () => void;
}

export function AgentPolicySwitch({
  checked,
  disabled,
  label,
  loading,
  onToggle,
}: AgentPolicySwitchProps) {
  return (
    <button
      aria-busy={loading}
      aria-checked={checked}
      aria-label={label}
      className={cn(
        "relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-60",
        checked ? "bg-[var(--color-primary)]" : "bg-[var(--color-surface-container-highest)]",
      )}
      disabled={disabled}
      role="switch"
      type="button"
      onClick={onToggle}
    >
      <span
        className={cn(
          "absolute size-4 rounded-full bg-[var(--color-elevated-surface-strong)] shadow transition-transform",
          checked ? "translate-x-[18px]" : "translate-x-0.5",
        )}
      />
    </button>
  );
}
