// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { createPortal } from "react-dom";
import { useState } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AgentMotion } from "@eliteforge/agent-ui-motion-react";
import { TronMotionProvider } from "./TronMotionProvider";
import { ShinyText } from "./ShinyText";
import { setMotionPreference, useMotionPreference } from "./motionPreferences";

let media: EventTarget & { matches: boolean };
beforeEach(() => {
  localStorage.clear();
  media = Object.assign(new EventTarget(), { matches: false });
  vi.stubGlobal("matchMedia", () => media);
  Object.defineProperty(document, "visibilityState", { configurable: true, value: "visible" });
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

function Settings() {
  const mode = useMotionPreference();
  const [draft, setDraft] = useState("");
  return <>
    <button onClick={() => setMotionPreference("off")}>Disable</button>
    <output>{mode}</output>
    <AgentMotion><input aria-label="Draft" value={draft} onChange={(event) => setDraft(event.target.value)} /></AgentMotion>
    <ShinyText>加载中…</ShinyText>
    {createPortal(<div data-slot="dialog-content">Portal</div>, document.body)}
  </>;
}

it("shares the persisted switch with labels, Agent UI and portal policy without losing drafts", () => {
  const { container } = render(<TronMotionProvider><Settings /></TronMotionProvider>);
  const input = screen.getByRole("textbox") as HTMLInputElement;
  fireEvent.change(input, { target: { value: "草稿" } });
  expect(container.querySelector("[data-tron-bits='enter']")).not.toBeNull();
  expect(screen.getByText("加载中…").getAttribute("data-motion")).toBe("full");
  fireEvent.click(screen.getByText("Disable"));
  expect(localStorage.getItem("tron.motion.mode")).toBe("off");
  expect(screen.getByText("加载中…").getAttribute("data-motion")).toBe("off");
  expect(document.documentElement.getAttribute("data-tron-motion-policy")).toBe("off");
  expect(input.value).toBe("草稿");
  expect(screen.getByRole("textbox")).toBe(input);
  expect(screen.getByText("Portal").parentElement).toBe(document.body);
});

it("reacts to system reduction, page visibility and cross-window preference updates", () => {
  render(<TronMotionProvider><ShinyText>Loading</ShinyText></TronMotionProvider>);
  const label = screen.getByText("Loading");
  act(() => { media.matches = true; media.dispatchEvent(new Event("change")); });
  expect(label.getAttribute("data-motion")).toBe("reduced");
  act(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, value: "hidden" });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  expect(label.getAttribute("data-motion")).toBe("off");
  act(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, value: "visible" });
    media.matches = false;
    document.dispatchEvent(new Event("visibilitychange"));
    localStorage.setItem("tron.motion.mode", "off");
    window.dispatchEvent(new StorageEvent("storage", { key: "tron.motion.mode" }));
  });
  expect(label.getAttribute("data-motion")).toBe("off");
});

it("falls back for invalid preferences and removes the portal policy on unmount", () => {
  localStorage.setItem("tron.motion.mode", "invalid");
  const { unmount } = render(<TronMotionProvider><ShinyText>Loading</ShinyText></TronMotionProvider>);
  expect(screen.getByText("Loading").getAttribute("data-motion")).toBe("full");
  unmount();
  expect(document.documentElement.hasAttribute("data-tron-motion-policy")).toBe(false);
});
