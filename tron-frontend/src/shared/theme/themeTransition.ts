import "./themeTransition.css";

let activeTransition: ViewTransition | undefined;
let revealGeneration = 0;

/** Reveal the new theme from the viewport's bottom centre over the old snapshot. */
export async function revealTheme(update: () => void): Promise<void> {
  const generation = ++revealGeneration;
  activeTransition?.skipTransition();
  activeTransition = undefined;
  const root = document.documentElement;
  const policy = root.dataset.tronMotionPolicy;
  if (typeof document.startViewTransition !== "function"
    || document.hidden
    || policy === "off"
    || policy === "reduced"
    || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    delete root.dataset.tronThemeTransition;
    root.style.removeProperty("--tron-theme-reveal-radius");
    update();
    return;
  }

  // The furthest points from bottom centre are the viewport's top corners.
  const radius = Math.ceil(Math.hypot(window.innerWidth / 2, window.innerHeight)) + 1;
  // circle() percentages use the reference box's normalized diagonal. Keeping
  // that coordinate space avoids Electron mis-scaling px radii on Retina.
  const referenceRadius = Math.hypot(window.innerWidth, window.innerHeight) / Math.SQRT2;
  root.style.setProperty("--tron-theme-reveal-radius", `${radius / referenceRadius * 100}%`);
  root.dataset.tronThemeTransition = "";

  const cleanup = () => {
    if (generation !== revealGeneration) return;
    activeTransition = undefined;
    delete root.dataset.tronThemeTransition;
    root.style.removeProperty("--tron-theme-reveal-radius");
  };
  let transition: ViewTransition;
  try {
    transition = document.startViewTransition(update);
    activeTransition = transition;
  } catch {
    cleanup();
    update();
    return;
  }

  const skip = () => transition.skipTransition();
  const onVisibilityChange = () => { if (document.hidden) skip(); };
  window.addEventListener("resize", skip);
  document.addEventListener("visibilitychange", onVisibilityChange);
  // A skipped animation rejects ready, but still applies the theme update.
  void transition.ready.catch(() => {});
  try {
    await transition.finished;
  } finally {
    window.removeEventListener("resize", skip);
    document.removeEventListener("visibilitychange", onVisibilityChange);
    cleanup();
  }
}
