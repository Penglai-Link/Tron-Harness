export const shellShortcutActions = [
  "newSession",
  "closeCurrentTab",
  "toggleLeftSidebar",
  "toggleRightSidebar",
  "toggleBottomPanel",
] as const;

export type ShellShortcutAction = (typeof shellShortcutActions)[number];

export interface ShellShortcutBinding {
  altKey: boolean;
  key: string;
  modKey: boolean;
  shiftKey: boolean;
}

export type ShellShortcutBindings = Record<
  ShellShortcutAction,
  ShellShortcutBinding | null
>;

export const CREATE_CHAT_SESSION_SHORTCUT_EVENT = "tron:create-chat-session";

const modifierKeys = new Set(["Alt", "Control", "Meta", "Shift"]);

interface KeyboardLockController {
  lock: (codes?: string[]) => Promise<void>;
  unlock: () => void;
}

const keyLabels: Record<string, string> = {
  " ": "Space",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  Backspace: "Backspace",
  Delete: "Delete",
  Enter: "Enter",
  Escape: "Esc",
  Tab: "Tab",
};

function normalizeKey(key: string) {
  if (key.length === 1) return key.toUpperCase();
  return key;
}

const codeToKeyMap: Record<string, string> = {
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
  Minus: "-",
  Equal: "=",
  Backquote: "`",
  Space: " ",
};

function codeToKey(code: string): string | undefined {
  if (!code) return undefined;
  const letterMatch = /^Key([A-Z])$/.exec(code);
  if (letterMatch) return letterMatch[1];
  const digitMatch = /^Digit([0-9])$/.exec(code);
  if (digitMatch) return digitMatch[1];
  return codeToKeyMap[code];
}

/**
 * Derives a stable key identifier from a keyboard event. Prefers
 * `event.code` (physical key) to avoid Mac Option-key dead keys and special
 * characters (e.g. Option+N reports `event.key === "Dead"`, Option+J reports
 * `event.key === "∆"`). Falls back to `event.key` when `event.code` is
 * unavailable or unmapped.
 */
function shortcutKeyFromEvent(
  event: Pick<KeyboardEvent, "code" | "key">,
): string | null {
  const codeKey = codeToKey(event.code);
  if (codeKey) return codeKey;
  if (!event.key || event.key === "Dead" || event.key === "Process") {
    return null;
  }
  return normalizeKey(event.key);
}

function isShortcutBinding(value: unknown): value is ShellShortcutBinding {
  if (!value || typeof value !== "object") return false;
  const binding = value as Partial<ShellShortcutBinding>;
  return typeof binding.key === "string"
    && Boolean(binding.key)
    && typeof binding.altKey === "boolean"
    && typeof binding.modKey === "boolean"
    && typeof binding.shiftKey === "boolean"
    && (binding.altKey || binding.modKey || binding.shiftKey);
}

export function createDefaultShellShortcutBindings(): ShellShortcutBindings {
  return {
    closeCurrentTab: null,
    newSession: null,
    toggleBottomPanel: {
      altKey: true,
      key: "J",
      modKey: true,
      shiftKey: false,
    },
    toggleLeftSidebar: {
      altKey: true,
      key: "[",
      modKey: true,
      shiftKey: false,
    },
    toggleRightSidebar: {
      altKey: true,
      key: "]",
      modKey: true,
      shiftKey: false,
    },
  };
}

export function normalizeShellShortcutBindings(value: unknown): ShellShortcutBindings {
  const defaults = createDefaultShellShortcutBindings();
  if (!value || typeof value !== "object") return defaults;
  const persisted = value as Partial<ShellShortcutBindings>;

  return shellShortcutActions.reduce<ShellShortcutBindings>((bindings, action) => {
    const binding = persisted[action];
    bindings[action] = isShortcutBinding(binding)
      ? {
          ...binding,
          key: normalizeKey(binding.key),
        }
      : defaults[action];
    return bindings;
  }, defaults);
}

export function shortcutFromKeyboardEvent(
  event: Pick<KeyboardEvent, "altKey" | "code" | "ctrlKey" | "key" | "metaKey" | "shiftKey">,
): ShellShortcutBinding | null {
  if (modifierKeys.has(event.key)) return null;
  const modKey = event.metaKey || event.ctrlKey;
  if (!modKey && !event.altKey && !event.shiftKey) return null;

  const key = shortcutKeyFromEvent(event);
  if (!key) return null;

  return {
    altKey: event.altKey,
    key,
    modKey,
    shiftKey: event.shiftKey,
  };
}

export function shortcutEquals(
  left: ShellShortcutBinding | null,
  right: ShellShortcutBinding | null,
) {
  return left?.altKey === right?.altKey
    && left?.key === right?.key
    && left?.modKey === right?.modKey
    && left?.shiftKey === right?.shiftKey;
}

export function shortcutMatchesKeyboardEvent(
  binding: ShellShortcutBinding | null,
  event: Pick<KeyboardEvent, "altKey" | "code" | "ctrlKey" | "key" | "metaKey" | "shiftKey">,
) {
  if (!binding) return false;
  const eventKey = shortcutKeyFromEvent(event);
  if (!eventKey) return false;
  return binding.altKey === event.altKey
    && binding.key === eventKey
    && binding.modKey === (event.metaKey || event.ctrlKey)
    && binding.shiftKey === event.shiftKey;
}

export function findMatchingShellShortcutAction(
  bindings: ShellShortcutBindings,
  event: Pick<KeyboardEvent, "altKey" | "code" | "ctrlKey" | "key" | "metaKey" | "shiftKey">,
) {
  return shellShortcutActions.find((action) => (
    shortcutMatchesKeyboardEvent(bindings[action], event)
  )) ?? null;
}

/**
 * Browser-reserved combinations such as Cmd+T are normally consumed before a
 * page receives keydown. Keyboard Lock can opt into receiving those events in
 * supported app containers/fullscreen contexts. Unsupported browsers simply
 * keep the normal page-level capture behavior.
 */
export function beginKeyboardShortcutCapture() {
  if (typeof navigator === "undefined") return () => undefined;
  const keyboard = (navigator as Navigator & { keyboard?: KeyboardLockController }).keyboard;
  if (!keyboard?.lock) return () => undefined;

  try {
    void keyboard.lock().catch(() => undefined);
  } catch {
    return () => undefined;
  }
  return () => {
    try {
      keyboard.unlock();
    } catch {
      // Ignore browsers that revoke keyboard lock when the page loses focus.
    }
  };
}

export function formatShortcutKeys(binding: ShellShortcutBinding | null, useMacSymbols = false) {
  if (!binding) return [];
  const modifiers = useMacSymbols
    ? [
        ...(binding.modKey ? ["⌘"] : []),
        ...(binding.altKey ? ["⌥"] : []),
        ...(binding.shiftKey ? ["⇧"] : []),
      ]
    : [
        ...(binding.modKey ? ["Ctrl"] : []),
        ...(binding.altKey ? ["Alt"] : []),
        ...(binding.shiftKey ? ["Shift"] : []),
      ];
  const key = keyLabels[binding.key] ?? binding.key;
  return [...modifiers, key];
}

export function formatShortcut(binding: ShellShortcutBinding | null, useMacSymbols = false) {
  return formatShortcutKeys(binding, useMacSymbols).join(useMacSymbols ? "" : " + ");
}

/**
 * Ctrl/Cmd/Alt combinations never insert text into editable surfaces, so
 * bindings carrying at least one of them stay active while the user types
 * (e.g. inside the chat composer). Shift-only bindings would swallow regular
 * input and stay inert on editable targets instead.
 */
export function hasSystemModifier(binding: ShellShortcutBinding | null) {
  return Boolean(binding?.modKey || binding?.altKey);
}

export function isEditableShortcutTarget(target: EventTarget | null) {
  if (typeof Element === "undefined" || !(target instanceof Element)) {
    return false;
  }
  return Boolean(
    target.closest("input, textarea, select, [contenteditable='true'], [role='textbox']"),
  );
}
