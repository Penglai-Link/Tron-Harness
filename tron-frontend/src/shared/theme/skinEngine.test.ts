// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { skinApi } from "./skinApi";
import { mountEffectLayers } from "./effectLayers";
import { prepareSkin, skinVariables } from "./skinEngine";
import type { Skin } from "./Skin";
import light from "./builtin/light.json";
import particles from "./migrations/matrix-particles.json";
import matrix from "./builtin/matrix-dark.json";
import { getTerminalAnsiTheme } from "@/components/terminal/terminalTheme";

vi.mock("@/components/particles-bg/particleAnimation", () => ({ animateParticles: vi.fn(() => vi.fn()) }));

vi.mock("./skinApi", () => ({ skinApi: { resource: vi.fn(async () => new Blob(["font"])) } }));
afterEach(() => { mountEffectLayers(new Set()); document.querySelectorAll("style[data-tron-skin], style[data-tron-skin-css]").forEach((style) => style.remove()); vi.unstubAllGlobals(); });
const custom = (): Skin => ({ id: "test", name: "Test", version: 1, mode: "Light", colors: {}, layout: {}, resources: { background: null, font: null } });

describe("skin runtime", () => {
  it("cleans up particles during preview, restore and disposal", async () => {
    const active = await prepareSkin(particles as Skin);
    const preview = await prepareSkin(light as Skin);
    active.apply();
    expect(document.querySelectorAll("[data-theme-particles]")).toHaveLength(1);
    preview.apply();
    expect(document.querySelectorAll("[data-theme-particles]")).toHaveLength(0);
    preview.dispose();
    active.apply();
    active.apply();
    expect(document.querySelectorAll("[data-theme-particles]")).toHaveLength(1);
    active.dispose();
    expect(document.querySelectorAll("[data-theme-particles]")).toHaveLength(0);
  });
  it("accepts all bundled manifests", () => {
    expect(() => skinVariables(particles as Skin)).not.toThrow();
    expect(() => skinVariables(light as Skin)).not.toThrow();
    expect(() => skinVariables(matrix as Skin)).not.toThrow();
  });
  it("bridges semantic colors to controls and gives layout the final value", () => {
    const skin = custom();
    skin.colors["--color-primary"] = "red";
    skin.layout["--color-primary"] = "blue";
    expect(skinVariables(skin)).toMatchObject({ "--color-primary": "blue", "--primary": "var(--color-primary)" });
  });
  it("bridges status colors and preserves explicit component and library overrides", () => {
    const skin = custom();
    skin.colors = { "--color-error": "#cc0000", "--color-primary": "#112233", "--primary": "#445566", "--tron-button-primary-bg": "#778899" };
    expect(skinVariables(skin)).toMatchObject({ "--destructive": "var(--color-error)", "--primary": "#445566", "--tron-button-primary-bg": "#778899" });
  });
  it("resolves custom ANSI colors without changing the remaining mode defaults", () => {
    document.documentElement.style.setProperty("--tron-terminal-red", "#aabbcc");
    try {
      expect(getTerminalAnsiTheme("light")).toMatchObject({ red: "#aabbcc", white: "#4b4b4b" });
    } finally { document.documentElement.style.removeProperty("--tron-terminal-red"); }
  });
  it("removes old variables and switches dark mode without a reload", async () => {
    const skin = custom(); skin.layout["--tron-card-radius"] = "24px";
    const first = await prepareSkin(skin); first.apply();
    expect(document.head.textContent).toContain("--tron-card-radius:24px");
    const next = await prepareSkin(matrix as Skin);
    first.dispose(); next.apply();
    expect(document.querySelectorAll("style[data-tron-skin]")).toHaveLength(1);
    expect(document.head.textContent).not.toContain("--tron-card-radius:24px");
    expect(document.documentElement.dataset.theme).toBe("matrix-dark");
    next.dispose();
    expect(document.querySelectorAll("style[data-tron-skin]")).toHaveLength(0);
  });
  it("rejects style injection before changing the active sheet", async () => {
    const active = await prepareSkin(light as Skin); active.apply();
    const bad = custom(); bad.layout["--x"] = "red;}body{display:none";
    await expect(prepareSkin(bad)).rejects.toThrow();
    expect(document.querySelector("style[data-tron-skin]")?.getAttribute("data-tron-skin")).toBe("light");
  });
  it("revokes local URLs and removes fonts on disposal and failed preparation", async () => {
    const createObjectURL = vi.fn(() => "blob:font"); const revokeObjectURL = vi.fn();
    vi.stubGlobal("URL", { createObjectURL, revokeObjectURL });
    const fonts = { add: vi.fn(), delete: vi.fn() };
    Object.defineProperty(document, "fonts", { value: fonts, configurable: true });
    class FakeFont { load = vi.fn(async () => this); }
    vi.stubGlobal("FontFace", FakeFont);
    const skin = custom(); skin.resources.font = "assets/font.woff2";
    const prepared = await prepareSkin(skin); prepared.apply(); prepared.dispose();
    expect(fonts.add).toHaveBeenCalledTimes(1); expect(fonts.delete).toHaveBeenCalledTimes(1);
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:font");
    vi.stubGlobal("FontFace", class { load = vi.fn(async () => { throw new Error("bad font"); }); });
    await expect(prepareSkin(skin)).rejects.toThrow("bad font");
    expect(revokeObjectURL).toHaveBeenCalledTimes(2);
  });
});


describe("effect CSS compilation", () => {
  function cssSkin(source: string) {
    vi.mocked(skinApi.resource).mockResolvedValue({ text: async () => source } as Blob);
    return { ...custom(), version: 2, resources: { css: "theme.css" } } as Skin;
  }
  it("compiles package image geometry and isolated animation names, then disposes layers and URLs", async () => {
    vi.stubGlobal("ResizeObserver", class { observe() {} unobserve() {} disconnect() {} });
    vi.stubGlobal("URL", { createObjectURL: () => "blob:vines", revokeObjectURL: vi.fn() });
    const source = '[data-theme-effect-layer="card"]::before, [data-theme-effect-layer="chat-message"]::after { position: absolute; inset: 0; pointer-events: none; background: url("assets/vines.png"); animation: sway 2s infinite; } @keyframes sway { from { transform: rotate(-1deg); } to { transform: rotate(1deg); } }';
    const prepared = await prepareSkin(cssSkin(source));
    const host = document.createElement("section"); host.dataset.themeAnchor = "card"; document.body.append(host);
    prepared.apply();
    const css = document.querySelector("style[data-tron-skin-css]")!.textContent;
    expect(css).toContain('body[data-tron-app][data-skin="test"] [data-theme-effect-layer="card"]::before');
    expect(css).toContain("test-sway");
    expect(css).toContain('url("blob:vines")');
    expect(host.querySelector('[data-theme-effect-layer="card"]')).not.toBeNull();
    prepared.dispose();
    expect(host.children).toHaveLength(0);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:vines"); host.remove();
  });
  it.each([
    'button { position: absolute; }',
    '[data-theme-effect-layer="card"] + button { width: 100px; }',
    '[data-theme-effect-layer="card"] { position: fixed; }',
    '[data-theme-effect-layer="card"] { pointer-events: auto; }',
    '[data-theme-effect-layer="card"] { opacity: 0 !important; }',
    '[data-theme-effect-layer="unknown"] { inset: 0; }',
    '@keyframes escape { to { position: fixed; } }',
    '[data-theme-effect-layer="card"] { background: url(https://example.com/vines.png); }',
    '[data-theme-effect-layer="card"] { button { width: 100px; } }',
  ])("rejects unsafe or layout-changing CSS: %s", async (source) => {
    await expect(prepareSkin(cssSkin(source))).rejects.toThrow();
  });
});
