// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode, useState } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ResolvedMotion } from "@eliteforge/agent-ui-motion-react";
import { AnimatedContent, FadeContent } from "./BitsContent";

const driver = vi.hoisted(() => ({ fromTo: vi.fn(), reverts: [] as ReturnType<typeof vi.fn>[] }));
vi.mock("gsap", () => ({ gsap: {
  fromTo: driver.fromTo,
  context: (callback: () => void) => {
    const revert = vi.fn();
    driver.reverts.push(revert);
    callback();
    return { revert };
  },
} }));

const motion: ResolvedMotion = { preset: "enter", enabled: true, duration: 180, delay: 40, distance: 8, intensity: 0.5, easing: "ease-out" };
function Draft() {
  const [value, setValue] = useState("");
  return <input aria-label="Draft" value={value} onChange={(event) => setValue(event.target.value)} />;
}

beforeEach(() => { driver.fromTo.mockClear(); driver.reverts.length = 0; });
afterEach(cleanup);

it("replays only explicit identity changes and keeps the input, draft and focus", () => {
  const content = (replayKey: number, duration = 180) => <AnimatedContent motion={{ ...motion, duration }} replayKey={replayKey}><Draft /></AnimatedContent>;
  const { rerender } = render(content(1));
  const input = screen.getByRole("textbox") as HTMLInputElement;
  input.focus();
  fireEvent.change(input, { target: { value: "unsaved draft" } });
  rerender(content(1, 240));
  expect(driver.fromTo).toHaveBeenCalledTimes(1);
  rerender(content(2));
  expect(driver.fromTo).toHaveBeenCalledTimes(2);
  expect(screen.getByRole("textbox")).toBe(input);
  expect(input.value).toBe("unsaved draft");
  expect(document.activeElement).toBe(input);
  expect(driver.reverts[0]).toHaveBeenCalledOnce();
  expect(driver.fromTo.mock.calls[1][1]).toEqual({ opacity: 0, y: 4 });
  expect(driver.fromTo.mock.calls[1][2]).toMatchObject({ duration: 0.18, delay: 0.04 });
});

it("cancels on disable, does not replay on re-enable, and cleans only owned contexts", () => {
  const content = (enabled: boolean, replayKey = 1) => <FadeContent motion={{ ...motion, preset: "fade", enabled }} replayKey={replayKey}><Draft /></FadeContent>;
  const { rerender, unmount } = render(content(true));
  expect(driver.fromTo.mock.calls[0][1]).toEqual({ opacity: 0, y: 0 });
  rerender(content(false));
  expect(driver.reverts[0]).toHaveBeenCalledOnce();
  rerender(content(true));
  expect(driver.fromTo).toHaveBeenCalledTimes(1);
  rerender(content(true, 2));
  unmount();
  expect(driver.reverts[1]).toHaveBeenCalledOnce();
});

it("leaves hidden or static content readable without starting a tween", () => {
  const { rerender } = render(<div hidden><AnimatedContent motion={motion}><Draft /></AnimatedContent></div>);
  expect(driver.fromTo).not.toHaveBeenCalled();
  rerender(<AnimatedContent motion={{ ...motion, enabled: false }}><Draft /></AnimatedContent>);
  expect(screen.getByRole("textbox").getAttribute("style")).toBeNull();
  expect(driver.fromTo).not.toHaveBeenCalled();
});

it("restores styles on completion and survives Strict Mode effect replay", () => {
  const { unmount } = render(<StrictMode><AnimatedContent motion={motion}><Draft /></AnimatedContent></StrictMode>);
  expect(driver.fromTo).toHaveBeenCalledTimes(2);
  expect(driver.reverts[0]).toHaveBeenCalledOnce();
  driver.fromTo.mock.calls[1][2].onComplete();
  expect(driver.reverts[1]).toHaveBeenCalledOnce();
  unmount();
  expect(driver.reverts[1]).toHaveBeenCalledOnce();
});
