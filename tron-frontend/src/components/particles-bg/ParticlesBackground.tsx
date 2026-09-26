import { useTheme } from "@/shared/theme/ThemeProvider";
import { useEffect, useRef } from "react";
import { animateParticles } from "./particleAnimation";

export function ParticlesBackground() {
  const { activeSkin } = useTheme();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    if (canvasRef.current) return animateParticles(canvasRef.current);
  }, [activeSkin]);
  return <canvas ref={canvasRef} aria-hidden="true" className="pointer-events-none fixed inset-0 z-0" />;
}
