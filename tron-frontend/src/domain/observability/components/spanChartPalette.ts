import { useEffect, useState } from "react";

// Canvas renderers need resolved colors; CSS variables alone are not inherited.
export function readSpanChartPalette() {
  const root = document.documentElement;
  const styles = getComputedStyle(root);
  const dark = root.dataset.theme === "matrix-dark";
  const color = (token: string, light: string, night: string) =>
    styles.getPropertyValue(token).trim() || (dark ? night : light);
  return {
    dark,
    text: color("--color-on-surface", "#1c1b1b", "#b5f3c8"),
    muted: color("--color-on-surface-variant", "#444748", "#8bb39a"),
    surface: color("--color-elevated-surface", "#ffffff", "#0b1a10"),
    outline: color("--color-outline-variant", "#c4c7c7", "#345a40"),
    primary: color("--color-primary", "#3478ce", "#73b6ff"),
    success: color("--color-success", "#16876b", "#78dfa0"),
    error: color("--color-error", "#ba1a1a", "#ff8585"),
  };
}

export function useSpanChartPalette() {
  const [palette, setPalette] = useState(readSpanChartPalette);
  useEffect(() => {
    const refresh = () => {
      const next = readSpanChartPalette();
      setPalette((previous) =>
        JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
    };
    const observer = new MutationObserver(refresh);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme", "data-skin", "style", "class"],
    });
    refresh();
    return () => observer.disconnect();
  }, []);
  return palette;
}
