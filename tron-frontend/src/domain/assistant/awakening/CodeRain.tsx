import { useEffect, useRef } from "react";

export function CodeRain({ active = true, departing = false }: { active?: boolean; departing?: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const departure = useRef<number | null>(null);
  useEffect(() => {
    departure.current = departing ? performance.now() : null;
  }, [departing]);
  useEffect(() => {
    if (!active) return;
    const el = canvas.current;
    if (!el) return;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    type Strip = {
      canvas: HTMLCanvasElement;
      context: CanvasRenderingContext2D;
      size: number;
      length: number;
      seed: number;
      brightness: number;
      nextMutation: number;
      width: number;
      height: number;
      alpha: number[];
    };
    type Drop = { x: number; y: number; speed: number; alpha: number; strip: Strip };
    let frame = 0, last = 0, w = 0, h = 0, ratio = 0;
    let strips: Strip[] = [], drops: Drop[] = [];
    let light = document.documentElement.dataset.theme === "light";
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const reduced = () => {
      const policy = document.documentElement.dataset.tronMotionPolicy;
      return policy === "off" || policy === "reduced" || (policy !== "full" && media.matches);
    };
    // Keep script families equally represented rather than letting the longest
    // alphabet dominate. Latin includes extended letters and Roman numerals.
    const glyphFamilies = [
      "天地玄黄宇宙洪荒光影星河万物心知觉醒未来山川风雨日月",
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
      "0123456789",
      "ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖØÙÚÛÜÝÞßæœøåⅠⅡⅢⅣⅤⅥⅦⅧⅨⅩ",
      "アイウエオカキクケコサシスセソタチツテトナニヌネノ",
      "<>:{}[]/+=∑∂∞λΩ",
    ];
    const paintCell = (strip: Strip, j: number, tick: number) => {
      const c = strip.context;
      const y = 8 + (strip.length - 1 - j) * strip.size;
      c.clearRect(0, y, strip.width, strip.size);
      c.globalAlpha = strip.alpha[j];
      c.fillStyle = light ? (j === 0 ? "#111" : j < 4 ? "#444" : "#777") : (j === 0 ? "#d2ffe1" : j < 4 ? "#68ff99" : "#14e157");
      const n = strip.seed + j * 17 + tick;
      const family = glyphFamilies[n % glyphFamilies.length];
      c.fillText(family[Math.floor(n / glyphFamilies.length) % family.length], 8, y);
    };
    const releaseStrips = () => {
      strips.forEach((strip) => { strip.canvas.width = strip.canvas.height = 0; });
      strips = [];
    };
    const makeStrip = (size: number, brightness: number): Strip => {
      const length = Math.round(28 + Math.random() * 34 + h / 45);
      const surface = document.createElement("canvas");
      const width = size + 16, height = length * size + 16;
      surface.width = Math.ceil(width * ratio);
      surface.height = Math.ceil(height * ratio);
      const context = surface.getContext("2d")!;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.font = `${size}px monospace`;
      context.textBaseline = "top";
      const strip: Strip = {
        canvas: surface, context, size, length, width, height, brightness,
        seed: Math.floor(Math.random() * 10000),
        nextMutation: performance.now() + Math.random() * 300,
        alpha: Array.from({ length }, (_, j) =>
          Math.min(1, Math.pow(1 - j / length, 1.35) * brightness + (j === 0 ? 0.2 : 0))),
      };
      // Bake the bright leading halo once; never run shadowBlur in the frame loop.
      if (!light && brightness > 0.5) {
        const glow = context.createRadialGradient(width / 2, height - size / 2 - 8, 0, width / 2, height - size / 2 - 8, size);
        glow.addColorStop(0, "#8affbd66");
        glow.addColorStop(1, "#8affbd00");
        context.fillStyle = glow;
        context.fillRect(0, height - size * 2 - 8, width, size * 2 + 8);
      }
      for (let j = 0; j < length; j++) paintCell(strip, j, 0);
      strips.push(strip);
      return strip;
    };
    const resize = () => {
      const nextRatio = Math.min(devicePixelRatio || 1, 1.5);
      if (w === el.clientWidth && h === el.clientHeight && ratio === nextRatio) return;
      w = el.clientWidth; h = el.clientHeight; ratio = nextRatio;
      el.width = Math.round(w * ratio); el.height = Math.round(h * ratio);
      ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
      releaseStrips();
      // A bounded pool keeps GPU memory independent of the number of columns.
      drops = [
        { spacing: 14, size: 10, speed: 38, brightness: 0.32 },
        { spacing: 18, size: 13, speed: 68, brightness: 0.58 },
        { spacing: 24, size: 16, speed: 105, brightness: 0.88 },
      ].flatMap((layer, depth) => {
        const pool = Array.from({ length: 12 }, () => makeStrip(layer.size, layer.brightness));
        return Array.from({ length: Math.ceil(w / layer.spacing) }, (_, i) => {
          const strip = pool[Math.floor(Math.random() * pool.length)];
          const x = i * layer.spacing + depth * 5;
          return {
            x, strip, y: Math.random() * (h + strip.length * strip.size),
            speed: layer.speed * (0.7 + Math.random() * 0.7),
            alpha: 1 - 0.46 * Math.exp(-Math.pow((x - w * 0.5) / Math.max(1, w * 0.25), 4)),
          };
        });
      });
      schedule();
    };
    const draw = (now: number) => {
      frame = 0;
      if (document.hidden) return;
      if (last && now - last < 1000 / 30 - 1) {
        frame = requestAnimationFrame(draw);
        return;
      }
      const dt = last ? Math.min((now - last) / 1000, 0.1) : 0;
      last = now;
      const still = reduced();
      const departureProgress = departure.current === null ? 0 : Math.min(1, (now - departure.current) / 2800);
      if (!still) {
        for (const strip of strips) {
          if (now < strip.nextMutation) continue;
          strip.nextMutation = now + 130 + strip.seed % 190;
          const tick = Math.floor(now / (130 + strip.seed % 190));
          // Flicker a few cells, not the entire column. Cached tails and halos
          // remain untouched while all columns continue moving smoothly.
          for (let k = 0; k < 3; k++) {
            const j = 1 + (tick * 13 + k * 17 + strip.seed) % (strip.length - 1);
            paintCell(strip, j, tick);
          }
        }
      }
      ctx.clearRect(0, 0, w, h);
      for (const d of drops) {
        if (!still) d.y += d.speed * dt * (1 + departureProgress * departureProgress * 7);
        const s = d.strip;
        if (departure.current === null && d.y - s.length * s.size > h) d.y = -s.size * 2;
        // Dissolve cached columns at different times, without regenerating glyphs
        // or adding another full-screen canvas during the route handoff.
        const dissolveStart = 0.08 + (s.seed % 100) / 100 * 0.3;
        const dissolve = Math.max(0, Math.min(1, (departureProgress - dissolveStart) / 0.6));
        ctx.globalAlpha = d.alpha * (1 - dissolve * dissolve * (3 - 2 * dissolve));
        ctx.drawImage(s.canvas, d.x - 8, d.y - (s.length - 1) * s.size - 8, s.width, s.height);
      }
      ctx.globalAlpha = 1;
      if (!still) frame = requestAnimationFrame(draw);
    };
    function schedule() {
      cancelAnimationFrame(frame);
      last = 0;
      if (!document.hidden) frame = requestAnimationFrame(draw);
    }
    const ro = new ResizeObserver(resize);
    const motionObserver = new MutationObserver(() => {
      const nextLight = document.documentElement.dataset.theme === "light";
      if (light !== nextLight) { light = nextLight; ratio = 0; resize(); }
      schedule();
    });
    motionObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-tron-motion-policy", "data-theme"] });
    ro.observe(el);
    resize();
    document.addEventListener("visibilitychange", schedule);
    media.addEventListener("change", schedule);
    return () => {
      cancelAnimationFrame(frame);
      ro.disconnect();
      motionObserver.disconnect();
      document.removeEventListener("visibilitychange", schedule);
      media.removeEventListener("change", schedule);
      releaseStrips();
      ctx.clearRect(0, 0, w, h);
    };
  }, [active]);
  return <canvas className="absolute inset-0 size-full opacity-30" ref={canvas} aria-hidden="true" />;
}
