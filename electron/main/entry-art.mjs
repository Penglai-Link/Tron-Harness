// Shared by the sandboxed Electron startup document and the React entry page.
// Keep this function self-contained so Electron can serialize it into its data URL.
export function mountEntryArtwork(canvas, { animated = true } = {}) {
  const context = canvas.getContext("2d");
  if (!context) return () => {};
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
  const colorScheme = matchMedia("(prefers-color-scheme: dark)");
  const pointer = { x: 0, y: 0, targetX: 0, targetY: 0 };
  let width = 0;
  let height = 0;
  let primary = "";
  let dark = false;
  let frame = 0;
  let lastFrame = 0;
  let elapsed = 0;
  let previousTime = 0;

  // u: along the fabric; v: across its fibers.
  // (u, v) -> twist + ripple -> perspective -> screen coordinates
  function point(u, v, time) {
    const scale = Math.min(width * 0.6, height * 1.15);
    const twist =
      u * 3.2 + 0.7 + Math.sin(time * 0.17) * 0.32 + pointer.x * 0.25;
    const ripple = Math.sin(u * 5 - time * 0.45) * 0.025 * (1 - v * v);
    const x = u * 1.2 + v * 0.08 * Math.sin(u * 2.2 + time * 0.12);
    const y =
      0.15 * Math.sin(u * 2.7 - time * 0.18) -
      0.16 * u +
      v * 0.28 * Math.cos(twist) +
      ripple +
      pointer.y * 0.06 * Math.cos(u * 2);
    const z =
      v * 0.4 * Math.sin(twist) + 0.11 * Math.cos(u * 2.6 + time * 0.17);
    const perspective = 2.8 / (2.8 - z);
    return [
      width * 0.59 + x * scale * perspective,
      height * 0.71 + y * scale * perspective,
    ];
  }

  function strand(v, time, reverse = false) {
    const samples = width < 600 ? 64 : 100;
    for (let step = 0; step <= samples; step++) {
      const u = (reverse ? 1 - step / samples : step / samples) * 2.6 - 1.3;
      const [x, y] = point(u, v, time);
      if (step === 0 && !reverse) context.moveTo(x, y);
      else context.lineTo(x, y);
    }
  }

  function draw(time) {
    context.clearRect(0, 0, width, height);
    const wash = context.createLinearGradient(
      width * 0.15,
      height,
      width,
      height * 0.25
    );
    wash.addColorStop(0, "transparent");
    wash.addColorStop(0.55, primary);
    wash.addColorStop(1, "transparent");
    context.fillStyle = wash;
    context.globalAlpha = dark ? 0.045 : 0.035;
    context.beginPath();
    strand(-1, time);
    strand(1, time, true);
    context.closePath();
    context.fill();

    context.strokeStyle = wash;
    const count = width < 600 ? 42 : 66;
    for (let line = 0; line < count; line++) {
      const v = (line / (count - 1)) * 2 - 1;
      context.globalAlpha = (dark ? 0.36 : 0.3) + Math.abs(v) * 0.13;
      context.lineWidth = line === 0 || line === count - 1 ? 1.25 : 0.7;
      context.beginPath();
      strand(v, time);
      context.stroke();
    }
    context.globalAlpha = 1;
  }

  function animate(timestamp) {
    frame = 0;
    if (document.hidden || reducedMotion.matches || !animated) return;
    if (timestamp - lastFrame >= 1000 / 30) {
      // Advance only while visible, so resuming never makes the fabric jump.
      elapsed += previousTime
        ? Math.min(timestamp - previousTime, 50) / 1000
        : 0;
      previousTime = timestamp;
      lastFrame = timestamp;
      pointer.x += (pointer.targetX - pointer.x) * 0.045;
      pointer.y += (pointer.targetY - pointer.y) * 0.045;
      draw(elapsed);
    }
    frame = requestAnimationFrame(animate);
  }

  function resume() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    previousTime = 0;
    pointer.x = pointer.y = 0;
    draw(elapsed);
    if (animated && !reducedMotion.matches && !document.hidden)
      frame = requestAnimationFrame(animate);
  }

  function resize() {
    width = innerWidth;
    height = innerHeight;
    const ratio = Math.min(devicePixelRatio || 1, 1.5);
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    refreshPalette();
    resume();
  }

  function refreshPalette() {
    const style = getComputedStyle(canvas);
    const nextPrimary = style.getPropertyValue("--entry-primary").trim();
    const nextDark = style.colorScheme === "dark";
    if (nextPrimary === primary && nextDark === dark) return;
    primary = nextPrimary;
    dark = nextDark;
    draw(elapsed);
  }

  const move = (event) => {
    pointer.targetX = (event.clientX / width) * 2 - 1;
    pointer.targetY = (event.clientY / height) * 2 - 1;
  };
  const leave = () => {
    pointer.targetX = pointer.targetY = 0;
  };
  addEventListener("resize", resize);
  addEventListener("pointermove", move, { passive: true });
  document.addEventListener("pointerleave", leave);
  document.addEventListener("visibilitychange", resume);
  reducedMotion.addEventListener("change", resume);
  colorScheme.addEventListener("change", resize);
  // The React skin may finish loading after the first frame. Refresh colors
  // without restarting the fabric or retaining the pre-theme fallback palette.
  const themeObserver = new MutationObserver(refreshPalette);
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme", "data-skin", "class", "style"],
  });
  resize();
  return () => {
    if (frame) cancelAnimationFrame(frame);
    removeEventListener("resize", resize);
    removeEventListener("pointermove", move);
    document.removeEventListener("pointerleave", leave);
    document.removeEventListener("visibilitychange", resume);
    reducedMotion.removeEventListener("change", resume);
    colorScheme.removeEventListener("change", resize);
    themeObserver.disconnect();
  };
}
