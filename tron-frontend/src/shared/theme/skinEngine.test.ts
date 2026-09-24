// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { prepareSkin, skinVariables } from "./skinEngine";
import type { Skin } from "./Skin";
import light from "./builtin/light.json";
import matrix from "./builtin/matrix-dark.json";
import { getTerminalAnsiTheme } from "@/components/terminal/terminalTheme";

vi.mock("./skinApi", () => ({ skinApi: { resource: vi.fn(async () => new Blob(["font"])) } }));
afterEach(() => { document.querySelectorAll("style[data-tron-skin]").forEach((style) => style.remove()); vi.unstubAllGlobals(); });
const custom = (): Skin => ({ id: "test", name: "Test", version: 1, mode: "Light", colors: {}, layout: {}, resources: { background: null, font: null } });

describe("skin runtime", () => {
  it("accepts both bundled manifests", () => {
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
