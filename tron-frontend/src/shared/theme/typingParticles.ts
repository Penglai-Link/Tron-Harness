export type TypingParticleStyle = "colorful" | "mosaic" | "neon" | "custom";
export type TypingParticleShape = "circle" | "square" | "diamond" | "star";

export interface TypingParticleSettings {
  style: TypingParticleStyle;
  shape: TypingParticleShape;
  count: number;
  size: number;
  speed: number;
  color: string;
}

export const defaultTypingParticleSettings: TypingParticleSettings = {
  style: "colorful",
  shape: "circle",
  count: 9,
  size: 2.6,
  speed: 1,
  color: "#46d9ff",
};

type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  hue: number;
  bornAt: number;
};

const particleLifetime = 620;
const mosaicHueOffsets = [-42, -20, 0, 20, 42];
let activeBurst: ((x: number, y: number) => void) | undefined;

function hueFromHex(color: string) {
  const value = Number.parseInt(color.slice(1), 16);
  const red = (value >> 16) / 255;
  const green = ((value >> 8) & 0xff) / 255;
  const blue = (value & 0xff) / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  if (delta === 0) return 0;
  const hue = max === red
    ? ((green - blue) / delta) % 6
    : max === green
      ? (blue - red) / delta + 2
      : (red - green) / delta + 4;
  return (hue * 60 + 360) % 360;
}

export function getTypingParticleColor(settings: TypingParticleSettings, hue: number) {
  if (settings.style === "custom" || settings.style === "neon") return settings.color;
  const baseHue = hueFromHex(settings.color);
  if (settings.style === "mosaic") {
    const shade = Math.abs(Math.round(hue)) % mosaicHueOffsets.length;
    return `hsl(${baseHue + mosaicHueOffsets[shade]} 88% ${[58, 62, 66, 62, 58][shade]}%)`;
  }
  return `hsl(${(baseHue + hue * 0.68) % 360} 92% 64%)`;
}

function drawShape(context: CanvasRenderingContext2D, shape: TypingParticleShape, x: number, y: number, size: number) {
  if (shape === "square") {
    context.fillRect(x - size, y - size, size * 2, size * 2);
    return;
  }
  context.beginPath();
  if (shape === "diamond") {
    context.moveTo(x, y - size);
    context.lineTo(x + size, y);
    context.lineTo(x, y + size);
    context.lineTo(x - size, y);
    context.closePath();
  } else if (shape === "star") {
    for (let point = 0; point < 10; point++) {
      const radius = point % 2 === 0 ? size : size * 0.45;
      const angle = -Math.PI / 2 + point * Math.PI / 5;
      const pointX = x + Math.cos(angle) * radius;
      const pointY = y + Math.sin(angle) * radius;
      if (point === 0) context.moveTo(pointX, pointY);
      else context.lineTo(pointX, pointY);
    }
    context.closePath();
  } else {
    context.arc(x, y, size, 0, Math.PI * 2);
  }
  context.fill();
}

/** Emit a burst at a viewport point when the appearance setting is enabled. */
export function emitTypingParticlesAt(x: number, y: number) {
  activeBurst?.(x, y);
}

export function createTypingParticleRenderer(
  canvas: HTMLCanvasElement,
  settings: TypingParticleSettings,
  viewportSized = false,
) {
  if (window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches) return null;
  const maybeContext = canvas.getContext("2d");
  if (!maybeContext) return null;
  const context: CanvasRenderingContext2D = maybeContext;
  const particles: Particle[] = [];
  let frame = 0;
  let disposed = false;
  let width = 0;
  let height = 0;

  function resize() {
    const bounds = canvas.getBoundingClientRect();
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    width = viewportSized ? window.innerWidth : bounds.width;
    height = viewportSized ? window.innerHeight : bounds.height;
    if (!width || !height) return;
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    if (viewportSized) {
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
    }
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
  }
  const resizeObserver = viewportSized || typeof ResizeObserver === "undefined"
    ? null
    : new ResizeObserver(resize);
  if (resizeObserver) resizeObserver.observe(canvas.parentElement ?? canvas);

  function tick(now: number) {
    if (disposed) return;
    context.clearRect(0, 0, width, height);
    for (let index = particles.length - 1; index >= 0; index--) {
      const particle = particles[index];
      const age = now - particle.bornAt;
      if (age >= particleLifetime) {
        particles.splice(index, 1);
        continue;
      }
      const progress = age / particleLifetime;
      particle.x += particle.vx;
      particle.y += particle.vy;
      particle.vy += 0.055 * settings.speed;
      const size = particle.size * (1 - progress * 0.35);
      context.globalAlpha = 1 - progress;
      if (settings.style === "mosaic") {
        context.fillStyle = getTypingParticleColor(settings, particle.hue);
        context.fillRect(particle.x - size, particle.y - size, size * 2, size * 2);
      } else if (settings.style === "neon") {
        context.fillStyle = getTypingParticleColor(settings, particle.hue);
        context.shadowBlur = size * 4;
        context.shadowColor = context.fillStyle;
        context.beginPath();
        context.arc(particle.x, particle.y, size * 0.75, 0, Math.PI * 2);
        context.fill();
        context.shadowBlur = 0;
      } else {
        context.fillStyle = getTypingParticleColor(settings, particle.hue);
        drawShape(
          context,
          settings.style === "custom" ? settings.shape : "circle",
          particle.x,
          particle.y,
          size * 0.72,
        );
      }
    }
    context.globalAlpha = 1;
    if (particles.length) frame = window.requestAnimationFrame(tick);
    else frame = 0;
  }

  function burst(x: number, y: number) {
    const now = performance.now();
    for (let index = 0; index < settings.count; index++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = (0.7 + Math.random() * 2.6) * settings.speed;
      particles.push({
        x,
        y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed - 1.2 * settings.speed,
        size: settings.size * (0.55 + Math.random() * 0.9),
        hue: settings.style === "mosaic"
          ? Math.floor(Math.random() * mosaicHueOffsets.length)
          : Math.random() * 360,
        bornAt: now,
      });
    }
    if (!frame) frame = window.requestAnimationFrame(tick);
  }

  resize();
  return {
    burst,
    resize,
    dispose() {
      disposed = true;
      resizeObserver?.disconnect();
      if (frame) window.cancelAnimationFrame(frame);
      context.clearRect(0, 0, width, height);
    },
  };
}

function readCaretPoint(target: HTMLInputElement | HTMLTextAreaElement | HTMLElement) {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    const bounds = target.getBoundingClientRect();
    const computed = getComputedStyle(target);
    const mirror = document.createElement("div");
    const marker = document.createElement("span");
    const isTextarea = target instanceof HTMLTextAreaElement;
    const styleProperties = [
      "boxSizing", "width", "height", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
      "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth", "fontFamily",
      "fontSize", "fontWeight", "fontStyle", "letterSpacing", "lineHeight", "textTransform",
      "textAlign", "textIndent", "wordSpacing",
    ] as const;
    mirror.style.position = "fixed";
    mirror.style.left = `${bounds.left}px`;
    mirror.style.top = `${bounds.top}px`;
    mirror.style.visibility = "hidden";
    mirror.style.overflow = "hidden";
    mirror.style.whiteSpace = isTextarea ? "pre-wrap" : "pre";
    mirror.style.overflowWrap = isTextarea ? "break-word" : "normal";
    for (const property of styleProperties) mirror.style[property] = computed[property];
    if (isTextarea) {
      mirror.scrollTop = target.scrollTop;
      mirror.scrollLeft = target.scrollLeft;
    }
    const caret = target.selectionStart ?? target.value.length;
    mirror.textContent = target.value.slice(0, caret);
    marker.textContent = "\u200b";
    mirror.appendChild(marker);
    document.body.appendChild(mirror);
    const markerBounds = marker.getBoundingClientRect();
    const point = { x: markerBounds.left, y: markerBounds.top + markerBounds.height / 2 };
    mirror.remove();
    return point;
  }

  const selection = window.getSelection();
  if (selection?.rangeCount) {
    const bounds = selection.getRangeAt(0).getBoundingClientRect();
    if (bounds.width || bounds.height) return { x: bounds.left, y: bounds.top + bounds.height / 2 };
  }
  const bounds = target.getBoundingClientRect();
  return { x: bounds.left + Math.min(bounds.width, 24), y: bounds.top + Math.min(bounds.height / 2, 24) };
}

function assistantTextPoint(message: HTMLElement) {
  const markdown = message.querySelector<HTMLElement>(".markdown-body");
  if (!markdown) return null;
  const walker = document.createTreeWalker(markdown, NodeFilter.SHOW_TEXT);
  let tail: Text | null = null;
  let current: Node | null;
  while ((current = walker.nextNode())) {
    if (current.textContent?.length) tail = current as Text;
  }
  if (!tail?.textContent?.length) return null;

  const range = document.createRange();
  range.setStart(tail, tail.textContent.length);
  range.collapse(true);
  const caret = range.getBoundingClientRect();
  if (caret.width || caret.height) {
    return { x: caret.left, y: caret.top + caret.height / 2 };
  }

  // Some engines report an empty rect for a collapsed range. Measure the last
  // rendered character and use its trailing edge as the caret position.
  const end = tail.textContent.length;
  const previousCodeUnit = tail.textContent.charCodeAt(end - 1);
  const start = previousCodeUnit >= 0xdc00 && previousCodeUnit <= 0xdfff ? end - 2 : end - 1;
  range.setStart(tail, Math.max(0, start));
  range.setEnd(tail, end);
  const glyph = range.getBoundingClientRect();
  if (!glyph.width && !glyph.height) return null;
  const rtl = getComputedStyle(markdown).direction === "rtl";
  return { x: rtl ? glyph.left : glyph.right, y: glyph.top + glyph.height / 2 };
}

/** Mount the caret, CodeMirror, and live assistant particle emitters. */
export function mountTypingParticles(
  enabled: boolean,
  settings: TypingParticleSettings = defaultTypingParticleSettings,
): () => void {
  if (!enabled || typeof document === "undefined") return () => {};
  if (window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches) return () => {};

  const canvas = document.createElement("canvas");
  canvas.dataset.typingParticles = "";
  canvas.setAttribute("aria-hidden", "true");
  canvas.style.cssText = "position:fixed;inset:0;pointer-events:none;z-index:9999";
  document.body.appendChild(canvas);
  const renderer = createTypingParticleRenderer(canvas, settings, true);
  if (!renderer) { canvas.remove(); return () => {}; }
  let disposed = false;
  const burstAt = (x: number, y: number) => renderer.burst(x, y);
  activeBurst = burstAt;

  function onInput(event: Event) {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    if (target.closest(".cm-editor")) return;
    const editable = target.isContentEditable || target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
    if (!editable) return;
    if (target instanceof HTMLInputElement && !["text", "search", "url", "email", "tel", "password", "number"].includes(target.type)) return;
    if (("readOnly" in target && target.readOnly) || ("disabled" in target && target.disabled)) return;

    window.requestAnimationFrame(() => {
      if (disposed || !target.isConnected) return;
      const point = readCaretPoint(target);
      burstAt(point.x, point.y);
    });
  }

  const lastAssistantBurst = new WeakMap<HTMLElement, number>();
  const onStreamMutation: MutationCallback = (mutations) => {
    for (const mutation of mutations) {
      const changed = mutation.target instanceof HTMLElement
        ? mutation.target
        : mutation.target.parentElement;
      const message = changed?.closest<HTMLElement>(
        '[data-theme-part="chat-message-assistant"][data-streaming="true"]',
      );
      if (!message || !changed || !message.querySelector(".markdown-body")?.contains(changed)) continue;
      const now = performance.now();
      if (now - (lastAssistantBurst.get(message) ?? 0) < 48) continue;
      const point = assistantTextPoint(message);
      if (!point) continue;
      lastAssistantBurst.set(message, now);
      burstAt(point.x, point.y);
    }
  };
  const streamObserver = new MutationObserver(onStreamMutation);
  streamObserver.observe(document.body, { subtree: true, childList: true, characterData: true });
  window.addEventListener("resize", renderer.resize);
  // Input events also fire for IME composition updates, unlike keydown events
  // that are commonly marked as composing and skipped by the browser.
  document.addEventListener("input", onInput, true);
  return () => {
    disposed = true;
    if (activeBurst === burstAt) activeBurst = undefined;
    window.removeEventListener("resize", renderer.resize);
    document.removeEventListener("input", onInput, true);
    streamObserver.disconnect();
    renderer.dispose();
    canvas.remove();
  };
}

const typingAccentStyles = `
@keyframes tron-typing-glow {
  0% { filter: drop-shadow(0 0 7px color-mix(in srgb, var(--color-primary) 72%, transparent)); }
  100% { filter: none; }
}
@keyframes tron-typing-ripple {
  from { opacity: .78; transform: translate(-50%, -50%) scale(.25); }
  to { opacity: 0; transform: translate(-50%, -50%) scale(2.5); }
}
[data-tron-typing-glow="true"] { animation: tron-typing-glow 420ms ease-out; }
.tron-typing-ripple {
  position: fixed; z-index: 10000; width: 18px; height: 18px; border: 1.5px solid var(--color-primary);
  border-radius: 50%; pointer-events: none; animation: tron-typing-ripple 380ms ease-out forwards;
}
@media (prefers-reduced-motion: reduce) {
  [data-tron-typing-glow="true"], .tron-typing-ripple { animation: none; }
}
`;

/** Mount a non-particle typing effect on editable fields and streaming replies. */
export function mountTypingEffect(
  effect: "off" | "particles" | "glow" | "ripple",
  settings: TypingParticleSettings = defaultTypingParticleSettings,
): () => void {
  if (effect === "off" || typeof document === "undefined") return () => {};
  if (effect === "particles") return mountTypingParticles(true, settings);
  if (window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches) return () => {};

  const style = document.createElement("style");
  style.dataset.tronTypingEffectStyles = "";
  style.textContent = typingAccentStyles;
  document.head.appendChild(style);
  const timers = new Set<number>();
  const lastStreamPulse = new WeakMap<HTMLElement, number>();
  let disposed = false;

  function schedule(callback: () => void, delay: number) {
    const timer = window.setTimeout(() => {
      timers.delete(timer);
      if (!disposed) callback();
    }, delay);
    timers.add(timer);
  }

  function pulse(element: HTMLElement) {
    element.dataset.tronTypingGlow = "true";
    schedule(() => {
      delete element.dataset.tronTypingGlow;
    }, 440);
  }

  function ripple(point: { x: number; y: number }) {
    const ring = document.createElement("span");
    ring.className = "tron-typing-ripple";
    ring.setAttribute("aria-hidden", "true");
    ring.style.left = `${point.x}px`;
    ring.style.top = `${point.y}px`;
    document.body.appendChild(ring);
    schedule(() => ring.remove(), 420);
  }

  function showAt(element: HTMLElement, point: { x: number; y: number }) {
    if (effect === "glow") pulse(element);
    else ripple(point);
  }

  function onInput(event: Event) {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const editor = target.closest<HTMLElement>(".cm-editor");
    const editable = editor || target.isContentEditable || target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
    if (!editable) return;
    if (("readOnly" in target && target.readOnly) || ("disabled" in target && target.disabled)) return;
    const pointTarget = target.closest(".cm-content") ?? target;
    window.requestAnimationFrame(() => {
      if (disposed || !target.isConnected) return;
      showAt(editor ?? target, readCaretPoint(pointTarget as HTMLElement));
    });
  }

  const onStreamMutation: MutationCallback = (mutations) => {
    for (const mutation of mutations) {
      const changed = mutation.target instanceof HTMLElement
        ? mutation.target
        : mutation.target.parentElement;
      const message = changed?.closest<HTMLElement>(
        '[data-theme-part="chat-message-assistant"][data-streaming="true"]',
      );
      if (!message || !changed || !message.querySelector(".markdown-body")?.contains(changed)) continue;
      const now = performance.now();
      if (now - (lastStreamPulse.get(message) ?? 0) < 64) continue;
      const point = assistantTextPoint(message);
      if (!point) continue;
      lastStreamPulse.set(message, now);
      showAt(message, point);
    }
  };

  const streamObserver = new MutationObserver(onStreamMutation);
  streamObserver.observe(document.body, { subtree: true, childList: true, characterData: true });
  document.addEventListener("input", onInput, true);
  return () => {
    disposed = true;
    document.removeEventListener("input", onInput, true);
    streamObserver.disconnect();
    timers.forEach((timer) => window.clearTimeout(timer));
    document.querySelectorAll(".tron-typing-ripple").forEach((ring) => ring.remove());
    document.querySelectorAll<HTMLElement>("[data-tron-typing-glow]").forEach((element) => {
      delete element.dataset.tronTypingGlow;
    });
    style.remove();
  };
}
