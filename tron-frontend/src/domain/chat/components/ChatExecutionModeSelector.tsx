import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ToolExecutionMode } from "@tron/chat-core";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface ChatExecutionModeSelectorProps {
  value: ToolExecutionMode;
  disabled?: boolean;
  onChange: (value: ToolExecutionMode) => void;
}

export function ChatExecutionModeSelector({ value, disabled, onChange }: ChatExecutionModeSelectorProps) {
  const { t } = useTranslation("pages");
  const options = [
    { value: "CONTROLLED", label: t("chat.management.composer.controlledExecution"), description: t("chat.management.composer.controlledExecutionHint") },
    { value: "AUTO_APPROVE", label: t("chat.management.composer.autoApprove"), description: t("chat.management.composer.autoApproveHint") },
  ] as const;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={t("chat.management.composer.executionMode")}
        disabled={disabled}
        className="inline-flex h-8 shrink-0 items-center gap-2 rounded-md px-2 text-xs text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] disabled:opacity-50"
      >
        <span>{options.find((option) => option.value === value)?.label}</span>
        <ChevronDown aria-hidden="true" className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="top" sideOffset={8} className="w-72 max-w-[calc(100vw-2rem)]">
        <DropdownMenuRadioGroup value={value} onValueChange={(next) => {
          if (next === "CONTROLLED" || next === "AUTO_APPROVE") onChange(next);
        }}>
          {options.map((option) => (
            <DropdownMenuRadioItem key={option.value} value={option.value} className="items-start py-2 pl-2">
              <span>
                <span className="block font-medium">{option.label}</span>
                <span className="mt-1 block text-xs text-muted-foreground">{option.description}</span>
              </span>
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
