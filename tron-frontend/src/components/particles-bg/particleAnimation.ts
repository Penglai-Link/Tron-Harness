export function animateParticles(canvas: HTMLCanvasElement) {
  const context = canvas.getContext("2d");
  if (!context) return;

  const color = getComputedStyle(document.documentElement).getPropertyValue("--color-on-surface").trim();
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const particles = Array.from({ length: 150 }, () => ({
    x: Math.random(),
    y: Math.random(),
    radius: Math.random() * 1.25 + 0.7,
    alpha: Math.random() * 0.5 + 0.25,
    dx: (Math.random() - 0.5) * 0.00008,
    dy: (Math.random() - 0.5) * 0.00008,
    translateX: 0,
    translateY: 0,
    magnetism: Math.random() * 3.5 + 0.5,
  }));
  const pointer = { x: 0.5, y: 0.5 };
  let frame = 0;

  const resize = () => {
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = window.innerWidth * ratio;
    canvas.height = window.innerHeight * ratio;
    canvas.style.width = `${window.innerWidth}px`;
    canvas.style.height = `${window.innerHeight}px`;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
  };
  const move = (event: PointerEvent) => {
    pointer.x = event.clientX / window.innerWidth;
    pointer.y = event.clientY / window.innerHeight;
  };
  const draw = () => {
    const width = window.innerWidth;
    const height = window.innerHeight;
    context.clearRect(0, 0, width, height);
    for (const particle of particles) {
      if (!reduceMotion) {
        particle.x = (particle.x + particle.dx + 1) % 1;
        particle.y = (particle.y + particle.dy + 1) % 1;
      }
      const mouseX = pointer.x * width - width / 2;
      const mouseY = pointer.y * height - height / 2;
      const targetX = mouseX / (10 / particle.magnetism);
      const targetY = mouseY / (10 / particle.magnetism);
      particle.translateX += (targetX - particle.translateX) / 80;
      particle.translateY += (targetY - particle.translateY) / 80;
      const x = particle.x * width + particle.translateX;
      const y = particle.y * height + particle.translateY;
      context.beginPath();
      context.arc(x, y, particle.radius, 0, Math.PI * 2);
      context.fillStyle = color;
      context.globalAlpha = particle.alpha;
      context.fill();
    }
    if (!reduceMotion) frame = window.requestAnimationFrame(draw);
  };

  resize();
  draw();
  window.addEventListener("resize", resize);
  window.addEventListener("pointermove", move, { passive: true });
  return () => {
    window.cancelAnimationFrame(frame);
    window.removeEventListener("resize", resize);
    window.removeEventListener("pointermove", move);
  };
}
