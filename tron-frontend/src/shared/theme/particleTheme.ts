import { animateParticles } from "@/components/particles-bg/particleAnimation";

let stopCurrent: (() => void) | undefined;

/** Keep one decorative canvas, including while previewing another theme. */
export function mountParticleTheme(enabled: boolean): () => void {
  stopCurrent?.();
  if (!enabled) return () => {};
  const canvas = document.createElement("canvas");
  canvas.dataset.themeParticles = "";
  canvas.setAttribute("aria-hidden", "true");
  canvas.style.cssText = "position:fixed;inset:0;pointer-events:none;z-index:10;opacity:0.55";
  document.body.appendChild(canvas);
  const stopAnimation = animateParticles(canvas);
  const stop = () => {
    stopAnimation?.();
    canvas.remove();
    if (stopCurrent === stop) stopCurrent = undefined;
  };
  stopCurrent = stop;
  return stop;
}
