import { describe, expect, it } from "vitest";

import {
  createDefaultShellShortcutBindings,
  findMatchingShellShortcutAction,
  formatShortcut,
  formatShortcutKeys,
  migrateDefaultShellShortcutBindings,
  normalizeShellShortcutBindings,
  shellShortcutActions,
  shortcutFromKeyboardEvent,
  shortcutMatchesKeyboardEvent,
  type ShellShortcutAction,
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

  it.each(["metaKey", "ctrlKey"] as const)("matches assigned defaults with %s", (modifier) => {
    const bindings = createDefaultShellShortcutBindings();
    const expected = {
      newSession: "N", toggleAgentWindow: "N", toggleVoiceInput: "M", closeCurrentTab: "W",
      previousTab: "H", nextTab: "L", closeAllTabs: "X", closeOtherTabs: "O",
      toggleLeftSidebar: "ArrowLeft", toggleRightSidebar: "ArrowRight", toggleBottomPanel: "ArrowDown",
    } satisfies Record<Exclude<ShellShortcutAction, "openNotifications">, string>;
    const assignedActions = shellShortcutActions.filter((item) => item !== "openNotifications");
    expect(Object.keys(expected).sort()).toEqual([...assignedActions].sort());
    for (const action of assignedActions) {
      expect(findMatchingShellShortcutAction(bindings, keyboardEvent({
        [modifier]: true, altKey: action !== "newSession", key: expected[action],
      }))).toBe(action);
    }
    expect(bindings.openNotifications).toBeNull();
    expect(formatShortcut(bindings.toggleBottomPanel)).toBe("Ctrl + Alt + ↓");
    expect(formatShortcutKeys(bindings.toggleBottomPanel, true)).toEqual(["⌘", "⌥", "↓"]);
  });

  it("upgrades the previous defaults and fills formerly unassigned actions", () => {
    const bindings = migrateDefaultShellShortcutBindings({
      newSession: { altKey: true, key: "N", modKey: true, shiftKey: false },
      toggleBottomPanel: { altKey: true, key: "K", modKey: true, shiftKey: false },
      toggleLeftSidebar: { altKey: true, key: "U", modKey: true, shiftKey: false },
      toggleRightSidebar: { altKey: true, key: "Y", modKey: true, shiftKey: false },
      toggleAgentWindow: null, closeAllTabs: null, closeCurrentTab: null,
      closeOtherTabs: null, previousTab: null, nextTab: null,
    });
    expect(bindings).toEqual(createDefaultShellShortcutBindings());
  });

  it("keeps explicitly cleared shortcuts disabled during normalization", () => {
    expect(normalizeShellShortcutBindings({ newSession: null }).newSession).toBeNull();
  });

  it("migrates built-in browser-reserved shortcuts but preserves custom bindings", () => {
    const bindings = migrateDefaultShellShortcutBindings({
      newSession: { altKey: false, key: "N", modKey: true, shiftKey: false },
      toggleBottomPanel: { altKey: true, key: "J", modKey: true, shiftKey: false },
      toggleLeftSidebar: { altKey: true, key: "[", modKey: true, shiftKey: false },
      toggleRightSidebar: { altKey: true, key: "]", modKey: true, shiftKey: false },
      toggleAgentWindow: { altKey: false, key: "A", modKey: true, shiftKey: true },
    });

    expect(bindings.newSession).toEqual(createDefaultShellShortcutBindings().newSession);
    expect(bindings.toggleBottomPanel).toEqual(createDefaultShellShortcutBindings().toggleBottomPanel);
    expect(bindings.toggleLeftSidebar).toEqual(createDefaultShellShortcutBindings().toggleLeftSidebar);
    expect(bindings.toggleRightSidebar).toEqual(createDefaultShellShortcutBindings().toggleRightSidebar);
    expect(bindings.toggleAgentWindow).toEqual({
      altKey: false,
      key: "A",
      modKey: true,
      shiftKey: true,
    });
  });

  it("does not expose the removed terminal action", () => {
    expect(shellShortcutActions).not.toContain("openTerminal");
  });

  it("uses event.code to avoid Mac Option dead keys (Option+N)", () => {
    // On Mac, Cmd+Option+N reports event.key === "Dead" but event.code === "KeyN".
    const macEvent = keyboardEvent({
      altKey: true,
      code: "KeyN",
      key: "Dead",
      metaKey: true,
    });

    expect(shortcutFromKeyboardEvent(macEvent)).toEqual({
      altKey: true,
      key: "N",
      modKey: true,
      shiftKey: false,
    });
  });

  it("matches default bindings via event.code when Mac Option produces special chars", () => {
    const bindings = createDefaultShellShortcutBindings();
    // Option+N emits a dead key; the physical key still identifies the Agent shortcut.
    const macEvent = keyboardEvent({
      altKey: true,
      code: "KeyN",
      ctrlKey: false,
      key: "Dead",
      metaKey: true,
    });

    expect(findMatchingShellShortcutAction(bindings, macEvent)).toBe("toggleAgentWindow");
  });

  it("rejects dead keys when event.code is unavailable", () => {
    expect(shortcutFromKeyboardEvent(keyboardEvent({
      altKey: true,
      code: "",
      key: "Dead",
    }))).toBeNull();
  });
});
