import { ChevronRight, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ShellShortcutAction } from "@/shared/lib/keyboardShortcuts";
import ActionButton from "./ActionButton";

interface SectionHeaderProps {
  label: string;
  open: boolean;
  onToggle: () => void;
  onCreate: () => void;
  createLabel: string;
  createDisabled?: boolean;
  shortcutAction: ShellShortcutAction;
}

export function SectionHeader({ label, open, onToggle, onCreate, createLabel, createDisabled, shortcutAction }: SectionHeaderProps) {
  const { t } = useTranslation("common");
  return (
    <div className="group/section flex h-7 min-w-0 items-center gap-1 rounded px-2 text-xs font-medium">
      <button
        type="button"
        aria-expanded={open}
        title={t(open ? "actions.collapse" : "actions.expand")}
        onClick={onToggle}
        className="flex h-full min-w-0 flex-1 items-center gap-1 text-left text-sidebar-foreground/50 transition-colors hover:text-sidebar-foreground focus-visible:rounded focus-visible:outline-2 focus-visible:outline-ring"
      >
        <span className="truncate">{label}</span>
        {!open && <ChevronRight size={12} className="shrink-0 opacity-60" aria-hidden="true" />}
      </button>
      <ActionButton
        aria-label={createLabel}
        shortcutAction={shortcutAction}
        disabled={createDisabled}
        onClick={onCreate}
        className="size-6 shrink-0 justify-center rounded text-sidebar-foreground/60 transition-opacity hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:outline-2 focus-visible:outline-ring disabled:opacity-30 motion-reduce:transition-none [@media(hover:hover)]:opacity-0 group-hover/section:opacity-100 group-focus-within/section:opacity-100"
      >
        <Plus size={15} aria-hidden="true" />
      </ActionButton>
    </div>
  );
}
