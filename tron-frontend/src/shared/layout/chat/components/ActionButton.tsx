import { useAppShellStore } from "@/store/appShellStore";
import { formatShortcut, type ShellShortcutAction } from "@/shared/lib/keyboardShortcuts";
import { cn } from "@/lib/utils";

interface ActionButtonProps {
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  className?: string;
  disabled?: boolean;
  shortcutAction?: ShellShortcutAction;
  children?: React.ReactNode;
  needHover?: boolean;
  "aria-label"?: string;
}

const ActionButton = ({
  disabled,
  shortcutAction,
  className,
  children,
  needHover,
  onClick,
  ...rest
}: ActionButtonProps) => {
  const binding = useAppShellStore((state) => shortcutAction ? state.shortcutBindings[shortcutAction] : null);
  const mac = /Mac|iPhone|iPad/i.test(navigator.userAgent);
  const shortcut = formatShortcut(binding, mac);
  const label = rest["aria-label"];
  return (
    <button
      type="button"
      title={shortcut ? `${label} (${shortcut})` : label}
      aria-keyshortcuts={binding ? [binding.modKey ? (mac ? "Meta" : "Control") : "", binding.altKey ? "Alt" : "", binding.shiftKey ? "Shift" : "", binding.key].filter(Boolean).join("+") : undefined}
      disabled={disabled}
      onClick={onClick}
      className={cn(`flex gap-1 items-center p-0 ${needHover ? 'opacity-60 hover:opacity-100' : ''}`, className)}
      {...rest}
    >
      {children ?? rest["aria-label"]}
    </button>
  );
};

export default ActionButton;
