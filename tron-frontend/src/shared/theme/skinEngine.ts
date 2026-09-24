import type { Skin } from "./Skin";
import { skinApi } from "./skinApi";

const aliases: Record<string, string> = {
  "--background": "--color-background", "--foreground": "--color-on-surface",
  "--card": "--color-elevated-surface", "--card-foreground": "--color-on-surface",
  "--popover": "--color-elevated-surface", "--popover-foreground": "--color-on-surface",
  "--primary": "--color-primary", "--primary-foreground": "--color-on-primary",
  "--secondary": "--color-secondary-container", "--secondary-foreground": "--color-on-secondary-container",
  "--accent": "--color-accent-container", "--accent-foreground": "--color-on-accent-container",
  "--muted": "--color-surface-container", "--muted-foreground": "--color-on-surface-variant",
  "--border": "--color-outline-variant", "--input": "--color-outline-variant", "--ring": "--color-focus-ring",
  "--sidebar": "--color-surface", "--sidebar-foreground": "--color-on-surface",
  "--sidebar-accent": "--color-accent-container", "--sidebar-accent-foreground": "--color-on-accent-container",
  "--sidebar-border": "--color-outline-variant", "--sidebar-ring": "--color-focus-ring",
  "--sidebar-primary": "--color-primary", "--sidebar-primary-foreground": "--color-on-primary",
  "--destructive": "--color-error", "--destructive-foreground": "--color-on-error",
  "--success": "--color-success", "--success-foreground": "--color-on-success",
  "--warning": "--color-warning", "--warning-foreground": "--color-on-warning",
  "--info": "--color-accent", "--info-foreground": "--color-on-accent",
};

export function skinVariables(skin: Skin): Record<string, string> {
  const variables = { ...skin.colors, ...skin.layout };
  for (const [key, value] of Object.entries(variables)) {
    if (!/^--[a-z0-9-]+$/.test(key) || key === "--tron-system-font-size"
      || typeof value !== "string" || /[;{}<>\\@]|url\s*\(|\/\*|!important/i.test(value)) {
      throw new Error("Invalid skin variable");
    }
  }
  for (const [alias, source] of Object.entries(aliases)) {
    if (!(alias in variables) && source in variables) variables[alias] = `var(${source})`;
  }
  return variables;
}

export interface PreparedSkin {
  apply: () => void;
  dispose: () => void;
}

export async function prepareSkin(skin: Skin): Promise<PreparedSkin> {
  const variables = skinVariables(skin);
  const urls: string[] = [];
  let font: FontFace | undefined;
  let style: HTMLStyleElement | undefined;
  const dispose = () => {
    style?.remove();
    if (font) document.fonts.delete(font);
    urls.forEach((url) => URL.revokeObjectURL(url));
  };
  const resourceUrl = async (path: string) => {
    if (path.startsWith("https://")) {
      const url = new URL(path);
      if (/["'\\\n\r<>]/.test(path) || url.username || url.password) throw new Error("Invalid skin URL");
      return path;
    }
    if (!path.split("/").every((part) => /^[a-zA-Z0-9_.-]+$/.test(part) && part !== "." && part !== "..")) {
      throw new Error("Invalid skin resource path");
    }
    const url = URL.createObjectURL(await skinApi.resource(skin.id, path));
    urls.push(url);
    return url;
  };
  try {
    if (skin.resources?.background) {
      const url = await resourceUrl(skin.resources.background);
      await new Promise<void>((resolve, reject) => {
        const image = new Image();
        const timer = window.setTimeout(() => reject(new Error("Skin background timed out")), 10000);
        image.onload = () => { clearTimeout(timer); resolve(); };
        image.onerror = () => { clearTimeout(timer); reject(new Error("Could not load skin background")); };
        image.src = url;
      });
      variables["--tron-skin-background"] = `url("${url}")`;
    }
    if (skin.resources?.font) {
      const url = await resourceUrl(skin.resources.font);
      font = new FontFace(`TronSkin-${skin.id}`, `url("${url}")`);
      const loadedFont = font;
      await new Promise<void>((resolve, reject) => {
        const timer = window.setTimeout(() => reject(new Error("Skin font timed out")), 10000);
        loadedFont.load().then(() => { clearTimeout(timer); resolve(); }, (reason) => { clearTimeout(timer); reject(reason); });
      });
      variables["--tron-skin-font"] = `"TronSkin-${skin.id}", sans-serif`;
      variables["--font-sans"] = variables["--tron-skin-font"];
    }
    return {
      dispose,
      apply() {
        if (font) document.fonts.add(font);
        style = document.createElement("style");
        style.dataset.tronSkin = skin.id;
        // Match the specificity of the static theme rules, and append after them.
        style.textContent = `:root[data-theme] {${Object.entries(variables).map(([key, value]) => `${key}:${value}`).join(";")}}`;
        document.head.appendChild(style);
        const root = document.documentElement;
        root.dataset.theme = skin.mode === "Dark" ? "matrix-dark" : "light";
        root.dataset.skin = skin.id;
        root.style.colorScheme = skin.mode === "Dark" ? "dark" : "light";
      },
    };
  } catch (error) { dispose(); throw error; }
}
