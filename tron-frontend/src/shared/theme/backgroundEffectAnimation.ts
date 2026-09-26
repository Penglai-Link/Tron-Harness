import { animateParticles } from "@/components/particles-bg/particleAnimation";
import type { BackgroundEffectSettings } from "./backgroundPreferences";

type DecorativePoint = {
  x: number;
  y: number;
  radius: number;
  phase: number;
  drift: number;
};

function drawSnowflake(context: CanvasRenderingContext2D, x: number, y: number, radius: number, angle: number) {
  context.save();
  context.translate(x, y);
  context.rotate(angle);
  context.strokeStyle = context.fillStyle;
  context.lineWidth = Math.max(0.9, radius * 0.2);
  context.lineCap = "round";
  for (let arm = 0; arm < 3; arm++) {
    const theta = arm * Math.PI / 3;
    const dx = Math.cos(theta) * radius;
    const dy = Math.sin(theta) * radius;
    context.beginPath();
    context.moveTo(-dx, -dy);
    context.lineTo(dx, dy);
    context.stroke();
    for (const direction of [-1, 1]) {
      const branchX = dx * direction * 0.58;
      const branchY = dy * direction * 0.58;
      const branchLength = radius * 0.35;
      for (const side of [-1, 1]) {
        const branchAngle = theta + Math.PI + side * Math.PI / 3;
        context.beginPath();
        context.moveTo(branchX, branchY);
        context.lineTo(branchX + Math.cos(branchAngle) * branchLength, branchY + Math.sin(branchAngle) * branchLength);
        context.stroke();
      }
    }
  }
  context.restore();
}

export function mountBackgroundEffect(canvas: HTMLCanvasElement, settings: BackgroundEffectSettings) {
  if (settings.effect === "off" || settings.effect === "aurora") return () => {};
  if (settings.effect === "particles") {
    return animateParticles(canvas, {
      count: settings.count,
      size: settings.size,
      color: settings.color,
      opacity: settings.opacity,
      speed: settings.speed,
    });
  }

  const context = canvas.getContext("2d");
  if (!context) return () => {};
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const points: DecorativePoint[] = Array.from({ length: settings.count }, () => ({
    x: Math.random(),
    y: Math.random(),
    radius: settings.effect === "snowfall"
      ? Math.min(12, settings.size * (1.5 + Math.random() * 1.5))
      : settings.effect === "stars"
        ? Math.min(6, settings.size * (0.9 + Math.random() * 0.8))
        : settings.size * (0.55 + Math.random() * 0.9),
    phase: Math.random() * Math.PI * 2,
    drift: (Math.random() - 0.5) * 0.15,
  }));
  let frame = 0;
  let width = 0;
  let height = 0;
  let disposed = false;

  function resize() {
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    width = window.innerWidth;
    height = window.innerHeight;
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    context!.setTransform(ratio, 0, 0, ratio, 0, 0);
  }

  function draw(now = 0) {
    if (disposed) return;
    context!.clearRect(0, 0, width, height);
    points.forEach((point) => {
      if (!reduceMotion && settings.effect === "snowfall") {
        point.y += (0.0001 + point.radius * 0.00004) * settings.speed;
        point.x += point.drift * 0.00005 * settings.speed;
        if (point.y > 1.02) { point.y = -0.02; point.x = Math.random(); }
        if (point.x < -0.02) point.x = 1.02;
        if (point.x > 1.02) point.x = -0.02;
      }
      const twinkle = settings.effect === "stars" && !reduceMotion
        ? 0.45 + Math.sin(now * 0.001 * settings.speed + point.phase) * 0.35
        : 1;
      const x = point.x * width;
      const y = point.y * height;
      context!.fillStyle = settings.color;
      context!.globalAlpha = settings.opacity * twinkle;
      if (settings.effect === "snowfall") {
        drawSnowflake(context!, x, y, point.radius, point.phase);
      } else {
        context!.beginPath();
        context!.arc(x, y, point.radius, 0, Math.PI * 2);
        if (settings.effect === "stars") {
          context!.shadowBlur = point.radius * 2;
          context!.shadowColor = settings.color;
        }
        context!.fill();
        context!.shadowBlur = 0;
      }
    });
    context!.globalAlpha = 1;
    if (!reduceMotion) frame = window.requestAnimationFrame(draw);
  }

  resize();
  draw();
  window.addEventListener("resize", resize);
  return () => {
    disposed = true;
    window.cancelAnimationFrame(frame);
    window.removeEventListener("resize", resize);
    context.clearRect(0, 0, width, height);
  };
}
