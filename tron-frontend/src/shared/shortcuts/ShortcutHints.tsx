import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Badge } from "@/components/ui/badge";
import { formatShortcut, type ShellShortcutAction } from "@/shared/lib/keyboardShortcuts";
import { useAppShellStore } from "@/store/appShellStore";

const ShortcutHintsContext = createContext(false);

export function ShortcutHintsProvider({ children }: { children: ReactNode }) {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let held = false;
    const reset = () => {
      held = false;
      clearTimeout(timer);
      timer = undefined;
      setVisible(false);
    };
    const update = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return reset();
      if (held) return;
      held = true;
      timer = setTimeout(() => setVisible(true), 400);
    };
    const visibility = () => { if (document.hidden) reset(); };
    window.addEventListener("keydown", update, true);
    window.addEventListener("keyup", update, true);
    window.addEventListener("blur", reset);
    document.addEventListener("visibilitychange", visibility);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", update, true);
      window.removeEventListener("keyup", update, true);
      window.removeEventListener("blur", reset);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, []);
  return <ShortcutHintsContext value={visible}>{children}</ShortcutHintsContext>;
}

/** A portal keeps key tips out of layout and outside clipped toolbars. */
export function ShortcutHint({ action, compact = false, align = "end" }: { action: ShellShortcutAction; compact?: boolean; align?: "start" | "end" }) {
  const visible = useContext(ShortcutHintsContext);
  const binding = useAppShellStore(state => state.shortcutBindings[action]);
  const anchor = useRef<HTMLSpanElement>(null);
  const [position, setPosition] = useState<{ top: number; right?: number; left?: number } | null>(null);
  useLayoutEffect(() => {
    if (!visible || !binding) return;
    const target = anchor.current?.parentElement;
    if (!target) return;
    const update = () => {
      const rect = target.getBoundingClientRect();
      if (target.checkVisibility?.() === false || target.closest('[hidden], [inert], [aria-hidden="true"]')
        || target.matches(':disabled, [aria-disabled="true"]') || !rect.width || !rect.height
        || rect.bottom <= 0 || rect.top >= window.innerHeight || rect.right <= 0 || rect.left >= window.innerWidth) {
        setPosition(null);
        return;
      }
      setPosition({
        top: Math.max(4, Math.min(window.innerHeight - 24, rect.bottom - 3)),
        ...(align === "start" ? { left: Math.max(4, rect.left + 2) } : { right: Math.max(4, window.innerWidth - rect.right + 2) }),
      });
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(target);
    const attributes = new MutationObserver(update);
    attributes.observe(target, { attributes: true, attributeFilter: ["disabled", "aria-disabled", "hidden", "class", "style"] });
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      observer.disconnect();
      attributes.disconnect();
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [visible, binding, align]);
  const mac = /Mac|iPhone|iPad/i.test(navigator.userAgent);
  // The held modifier is implicit on compact icon controls; keep Shift/Alt.
  const label = formatShortcut(binding && compact ? { ...binding, modKey: false } : binding, compact || mac);
  return <>
    <span ref={anchor} hidden aria-hidden="true" />
    {visible && binding && position ? createPortal(
      <Badge variant="outline" size="sm" aria-hidden="true" data-shortcut-hint={action}
        className="pointer-events-none fixed z-[120] h-5 min-w-5 border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-1 text-[10px] leading-none text-[var(--color-on-surface)] shadow-sm"
        style={position}>{label}</Badge>, document.body,
    ) : null}
  </>;
}
