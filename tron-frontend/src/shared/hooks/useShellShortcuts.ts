import { useEffect, useRef } from "react";

import {
  findMatchingShellShortcutAction,
  hasSystemModifier,
  isEditableShortcutTarget,
  type ShellShortcutAction,
} from "@/shared/lib/keyboardShortcuts";
import { useAppShellStore } from "@/store/appShellStore";

/**
 * Consumes one shell shortcut action. Returning `false` leaves the keyboard
 * event untouched (no preventDefault) when the action does not apply to the
 * current context.
 */
export type ShellShortcutHandler = (event: KeyboardEvent) => false | void;

export type ShellShortcutHandlers = Partial<
  Record<ShellShortcutAction, ShellShortcutHandler>
>;

/**
 * Wires the shortcut bindings configured on the shortcuts settings page to
 * the handlers owned by the shell. Handlers are refreshed on every render, so
 * they can close over shell state freely without stale captures. Adding a new
 * shortcut only requires extending `shellShortcutActions` and passing one
 * extra handler entry.
 */
export function useShellShortcuts(handlers: ShellShortcutHandlers) {
  const shortcutBindings = useAppShellStore((state) => state.shortcutBindings);
  const handlersRef = useRef(handlers);

  useEffect(() => {
    handlersRef.current = handlers;
  });

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        event.defaultPrevented
        || event.isComposing
        || event.repeat
      ) {
        return;
      }

      const action = findMatchingShellShortcutAction(shortcutBindings, event);
      if (!action) return;

      const handler = handlersRef.current[action];
      if (!handler) return;

      // Bindings built only from Shift would swallow regular typing, so they
      // stay inert while an editable surface has focus. Ctrl/Cmd/Alt combos
      // never insert text and stay available, e.g. inside the chat composer.
      if (
        isEditableShortcutTarget(event.target)
        && !hasSystemModifier(shortcutBindings[action])
      ) {
        return;
      }

      if (handler(event) === false) return;
      event.preventDefault();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [shortcutBindings]);
}
