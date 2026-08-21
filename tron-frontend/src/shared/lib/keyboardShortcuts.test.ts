import { describe, expect, it } from "vitest";

import {
  createDefaultShellShortcutBindings,
  findMatchingShellShortcutAction,
  formatShortcut,
  formatShortcutKeys,
  shellShortcutActions,
  shortcutFromKeyboardEvent,
  shortcutMatchesKeyboardEvent,
} from "./keyboardShortcuts";

function keyboardEvent(overrides: Partial<KeyboardEvent> = {}) {
  return {
    altKey: false,
    ctrlKey: false,
    key: "j",
    metaKey: false,
    shiftKey: false,
    ...overrides,
  } as KeyboardEvent;
}

describe("keyboard shortcuts", () => {
  it("captures a portable modifier binding", () => {
    expect(shortcutFromKeyboardEvent(keyboardEvent({ altKey: true, ctrlKey: true }))).toEqual({
      altKey: true,
      key: "J",
      modKey: true,
      shiftKey: false,
    });
  });

  it("requires at least one modifier and ignores modifier keys themselves", () => {
    expect(shortcutFromKeyboardEvent(keyboardEvent())).toBeNull();
    expect(shortcutFromKeyboardEvent(keyboardEvent({ key: "Control" }))).toBeNull();
  });

  it("matches only the configured modifier set", () => {
    const binding = shortcutFromKeyboardEvent(keyboardEvent({ altKey: true, ctrlKey: true }));

    expect(shortcutMatchesKeyboardEvent(binding, keyboardEvent({ altKey: true, ctrlKey: true }))).toBe(true);
    expect(shortcutMatchesKeyboardEvent(binding, keyboardEvent({ ctrlKey: true }))).toBe(false);
  });

  it("ships the requested panel toggles with distinct defaults", () => {
    const bindings = createDefaultShellShortcutBindings();

    expect(findMatchingShellShortcutAction(bindings, keyboardEvent({ altKey: true, ctrlKey: true, key: "[" }))).toBe("toggleLeftSidebar");
    expect(findMatchingShellShortcutAction(bindings, keyboardEvent({ altKey: true, ctrlKey: true, key: "]" }))).toBe("toggleRightSidebar");
    expect(findMatchingShellShortcutAction(bindings, keyboardEvent({ altKey: true, ctrlKey: true, key: "j" }))).toBe("toggleBottomPanel");
    expect(formatShortcut(bindings.toggleBottomPanel)).toBe("Ctrl + Alt + J");
    expect(formatShortcutKeys(bindings.toggleBottomPanel, true)).toEqual(["⌘", "⌥", "J"]);
  });

  it("does not expose the removed terminal action", () => {
    expect(shellShortcutActions).not.toContain("openTerminal");
  });
});
