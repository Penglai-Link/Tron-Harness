import type { Skin } from "./Skin";
import { skinApi } from "./skinApi";
import postcss from "postcss";

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

const visualProperties = new Set(("color background background-color background-image background-position background-size background-repeat background-blend-mode border border-color border-style border-width border-radius outline outline-color outline-style outline-width box-shadow text-shadow opacity filter backdrop-filter mix-blend-mode animation animation-name animation-duration animation-delay animation-direction animation-fill-mode animation-iteration-count animation-play-state animation-timing-function transition transition-property transition-duration transition-delay transition-timing-function transform content font-family font-style font-weight text-decoration text-decoration-color text-decoration-style text-decoration-line -webkit-text-stroke -webkit-text-fill-color").split(/\s+/));
const safeCssFunctions = new Set("url var rgb rgba hsl hsla hwb lab lch oklab oklch color color-mix linear-gradient repeating-linear-gradient radial-gradient repeating-radial-gradient conic-gradient repeating-conic-gradient blur brightness contrast drop-shadow grayscale hue-rotate invert opacity saturate sepia translate translatex translatey translatez translate3d scale scalex scaley scale3d rotate rotatex rotatey rotatez rotate3d skew skewx skewy matrix matrix3d cubic-bezier steps calc min max clamp".split(" "));

async function compileSkinCss(source: string, skin: Skin, resourceUrl: (path: string) => Promise<string>): Promise<string> {
  let root: postcss.Root;
  try { root = postcss.parse(source, { from: skin.resources.css ?? "theme.css" }); }
  catch (error) { throw new Error(`主题 CSS ${skin.resources.css ?? "theme.css"}: ${error instanceof Error ? error.message : String(error)}`); }
  const scope = `body[data-tron-app][data-skin="${skin.id}"]`;
  const keyframes = new Map<string, string>();
  root.walkAtRules((rule) => {
    if (!/^(-webkit-)?keyframes$/i.test(rule.name)) throw new Error(`主题 CSS ${skin.resources.css}: 禁止 @${rule.name}`);
    const original = rule.params.trim();
    const renamed = `${skin.id}-${original}`;
    keyframes.set(original, renamed);
    rule.params = renamed;
  });
  const resources = new Map<string, string>();
  const rules: postcss.Rule[] = [];
  root.walkRules((rule) => rules.push(rule));
  for (const rule of rules) {
    const parent = rule.parent;
    const isKeyframe = parent?.type === "atrule" && /^(-webkit-)?keyframes$/i.test(parent.name);
    const isEffectLayer = !isKeyframe && rule.selectors.length === 1
      && /^\[data-theme-effect-layer(?:\s*=\s*(['"]?)(?:global|chat)\1)?\](?:::before|::after)?$/i.test(rule.selectors[0].trim());
    if (!isKeyframe) {
      rule.selectors = rule.selectors.map((selector) => `${scope} ${selector.trim()}`);
    }
    rule.walkDecls((decl) => {
      const overlayGeometry = isEffectLayer && /^(?:position|inset|inset-block|inset-inline|top|right|bottom|left|width|height|min-width|min-height|max-width|max-height)$/i.test(decl.prop);
      if (!decl.prop.startsWith("--") && !visualProperties.has(decl.prop.toLowerCase()) && !overlayGeometry) throw new Error(`主题 CSS ${skin.resources.css}: 属性 ${decl.prop} 会改变组件尺寸或排布`);
      if (/^(?:animation|animation-name)$/i.test(decl.prop)) {
        for (const [name, renamed] of keyframes) {
          const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
          decl.value = decl.value.replace(new RegExp(`(?<![\\w-])${escapedName}(?![\\w-])`, "g"), renamed);
        }
      }
      if (decl.value.includes("\\")) throw new Error(`主题 CSS ${skin.resources.css}: CSS 转义不受支持`);
      if (/expression\s*\(|javascript:|behavior\s*:|!important/i.test(decl.value)) throw new Error(`主题 CSS ${skin.resources.css}: 拒绝危险声明 ${decl.prop}`);
      for (const match of decl.value.matchAll(/([a-z-]+)\s*\(/gi)) if (!safeCssFunctions.has(match[1].toLowerCase())) throw new Error(`主题 CSS ${skin.resources.css}: 不支持 CSS 函数 ${match[1]}`);
      for (const match of decl.value.matchAll(/url\(\s*(['"]?)([^)'"\s]+)\1\s*\)/gi)) {
        const path = match[2];
        if (/^(?:https?:|data:|\/|\\|#)/i.test(path)) throw new Error(`主题 CSS ${skin.resources.css}: 资源必须来自主题包：${path}`);
        if (!resources.has(path)) resources.set(path, "");
      }
    });
  }
  for (const path of resources.keys()) resources.set(path, await resourceUrl(path));
  root.walkDecls((decl) => {
    decl.value = decl.value.replace(/url\(\s*(['"]?)([^)'"\s]+)\1\s*\)/gi, (_match, _quote: string, path: string) => `url("${resources.get(path)}")`);
  });
  return root.toString();
}

export async function prepareSkin(skin: Skin): Promise<PreparedSkin> {
  const variables = skinVariables(skin);
  const urls: string[] = [];
  let font: FontFace | undefined;
  let style: HTMLStyleElement | undefined;
  let customCssStyle: HTMLStyleElement | undefined;
  const dispose = () => {
    style?.remove();
    customCssStyle?.remove();
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
    if (skin.resources?.css) {
      if (skin.version < 2) throw new Error(`主题格式版本 ${skin.version} 不支持 CSS`);
      const css = await (await skinApi.resource(skin.id, skin.resources.css)).text();
      if (new TextEncoder().encode(css).length > 1024 * 1024) throw new Error(`主题 CSS ${skin.resources.css} 超过 1 MiB`);
      if (css.includes("\\")) throw new Error(`主题 CSS ${skin.resources.css}: CSS 转义不受支持`);
      const customCss = await compileSkinCss(css, skin, async (path) => {
        if (!path.split("/").every((part) => /^[a-zA-Z0-9_.-]+$/.test(part) && part !== "." && part !== "..")) throw new Error(`主题 CSS ${skin.resources.css}: 非法资源路径 ${path}`);
        const url = URL.createObjectURL(await skinApi.resource(skin.id, path));
        urls.push(url);
        return url;
      });
      customCssStyle = document.createElement("style");
      customCssStyle.dataset.tronSkinCss = skin.id;
      customCssStyle.textContent = customCss;
    }
    return {
      dispose,
      apply() {
        if (font) document.fonts.add(font);
        style?.remove();
        customCssStyle?.remove();
        style = document.createElement("style");
        style.dataset.tronSkin = skin.id;
        // Match the specificity of the static theme rules, and append after them.
        style.textContent = `:root[data-theme] {${Object.entries(variables).map(([key, value]) => `${key}:${value}`).join(";")}}`;
        document.head.appendChild(style);
        if (customCssStyle) document.head.appendChild(customCssStyle);
        const root = document.documentElement;
        root.dataset.theme = skin.mode === "Dark" ? "matrix-dark" : "light";
        root.dataset.skin = skin.id;
        document.body.dataset.skin = skin.id;
        root.style.colorScheme = skin.mode === "Dark" ? "dark" : "light";
      },
    };
  } catch (error) { dispose(); throw error; }
}
