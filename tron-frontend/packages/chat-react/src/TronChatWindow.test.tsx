// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { defaultFloatingState } from "@tron/chat-core";
import { ensureTestLocalStorage } from "./testLocalStorage";
import { TronChatWindow } from "./TronChatWindow";

beforeEach(() => {
  ensureTestLocalStorage();
  Object.defineProperty(HTMLElement.prototype, "setPointerCapture", {
    configurable: true,
    value: vi.fn(),
    writable: true,
  });
  Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", {
    configurable: true,
    value: vi.fn(),
    writable: true,
  });
  Object.defineProperty(HTMLElement.prototype, "scrollTo", {
    configurable: true,
    value: vi.fn(),
    writable: true,
  });
});

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

function renderWindow(props: Partial<ComponentProps<typeof TronChatWindow>> = {}) {
  return render(
    <TronChatWindow
      createSessionLabel="Create session"
      draft=""
      messages={[]}
      onCreateSession={() => undefined}
      onDraftChange={() => undefined}
      onSend={() => undefined}
      sendDisabled
      session={null}
      title="Chat"
      titleSubtitle="Workspace assistant"
      {...props}
    />,
  );
}

describe("TronChatWindow", () => {
  it("persists collapse state in uncontrolled mode", () => {
    renderWindow({ storageKey: "tron.chat.window.test.uncontrolled" });

    fireEvent.click(screen.getByRole("button", { name: "Collapse chat" }));

    expect(screen.getByRole("button", { name: "Expand chat" })).toBeTruthy();
    expect(window.localStorage.getItem("tron.chat.window.test.uncontrolled")).toContain("\"collapsed\":true");
  });

  it("delegates state changes through onStateChange in controlled mode", () => {
    const onStateChange = vi.fn();
    renderWindow({
      onStateChange,
      state: defaultFloatingState,
    });

    fireEvent.click(screen.getByRole("button", { name: "Collapse chat" }));

    expect(onStateChange).toHaveBeenCalledTimes(1);
    expect(onStateChange).toHaveBeenCalledWith({
      ...defaultFloatingState,
      collapsed: true,
    });
    expect(window.localStorage.length).toBe(0);
  });

  it("normalizes controlled floating state into the viewport on mount", () => {
    const onStateChange = vi.fn();
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 800,
      writable: true,
    });
    Object.defineProperty(window, "innerHeight", {
      configurable: true,
      value: 640,
      writable: true,
    });

    renderWindow({
      onStateChange,
      state: {
        ...defaultFloatingState,
        height: 520,
        x: 400,
        y: 200,
      },
    });

    expect(onStateChange).toHaveBeenCalledTimes(1);
    expect(onStateChange).toHaveBeenCalledWith({
      ...defaultFloatingState,
      height: 520,
      x: 324,
      y: 84,
    });
  });
});
