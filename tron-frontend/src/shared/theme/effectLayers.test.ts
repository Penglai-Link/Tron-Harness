// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { effectLayerNames, effectSelectorName, mountEffectLayers } from "./effectLayers";

const flush = () => new Promise((resolve) => setTimeout(resolve, 35));
beforeEach(() => {
  vi.stubGlobal("ResizeObserver", class { observe() {} unobserve() {} disconnect() {} });
  document.body.innerHTML = '<section data-theme-anchor="card"><button>Open</button></section><section data-theme-anchor="card"></section>';
});
afterEach(() => { mountEffectLayers(new Set()); document.body.innerHTML = ""; vi.unstubAllGlobals(); });

describe("theme effect anchors", () => {
  it("recognizes only standalone supported targets, never components related by selectors", () => {
    for (const name of effectLayerNames) expect(effectSelectorName(`[data-theme-effect-layer="${name}"]::before`)).toBe(name);
    for (const selector of ['[data-theme-effect-layer="card"] + button', ':is([data-theme-effect-layer="card"], button)', '[data-theme-effect-layer-evil]', '[data-theme-effect-layer="unknown"]']) expect(effectSelectorName(selector)).toBeNull();
  });
  it("creates independent inert decorations, tracks insertion/removal, and fully cleans up", async () => {
    const stop = mountEffectLayers(new Set(["card"]));
    expect(document.querySelectorAll('[data-theme-effect-layer="card"]')).toHaveLength(2);
    const viewport = document.querySelector<HTMLElement>('[data-theme-effect-viewport]')!;
    expect(viewport.getAttribute("aria-hidden")).toBe("true");
    expect(viewport.inert).toBe(true);
    expect(viewport.style.pointerEvents).toBe("none");
    const first = document.querySelector("section")!;
    first.insertAdjacentHTML("afterend", '<section data-theme-anchor="card"></section>');
    await flush();
    expect(document.querySelectorAll('[data-theme-effect-layer="card"]')).toHaveLength(3);
    first.remove();
    await flush();
    expect(first.querySelector('[data-theme-effect-viewport]')).toBeNull();
    stop();
    expect(document.querySelector('[data-theme-effect-viewport]')).toBeNull();
    expect(document.querySelector('[data-theme-effect-positioned]')).toBeNull();
  });
  it("replaces layers on preview/theme changes without letting an old disposer remove the new theme", () => {
    const old = mountEffectLayers(new Set(["card"]));
    const stopPreview = mountEffectLayers(new Set(["global"]));
    old();
    expect(document.querySelectorAll('[data-theme-effect-layer]')).toHaveLength(1);
    expect(document.querySelector('[data-theme-effect-layer="global"]')).not.toBeNull();
    stopPreview();
    mountEffectLayers(new Set(["card"]));
    expect(document.querySelectorAll('[data-theme-effect-layer="card"]')).toHaveLength(2);
  });
  it("pins decoration to a scrolled host viewport and keeps existing positioning", () => {
    const host = document.querySelector<HTMLElement>("section")!;
    host.style.position = "fixed";
    Object.defineProperties(host, { clientWidth: { value: 240 }, clientHeight: { value: 100 } });
    mountEffectLayers(new Set(["card"]));
    host.scrollTop = 75;
    host.dispatchEvent(new Event("scroll"));
    const viewport = host.querySelector<HTMLElement>('[data-theme-effect-viewport]')!;
    expect(viewport.style.top).toBe("75px");
    expect(viewport.style.width).toBe("240px");
    expect(viewport.style.height).toBe("100px");
    expect(host.hasAttribute("data-theme-effect-positioned")).toBe(false);
  });
});
