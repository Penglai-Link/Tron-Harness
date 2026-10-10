import { useLayoutEffect, useMemo, type ReactNode } from "react";
import { MotionProvider, useMotion } from "@eliteforge/agent-ui-motion-react";
import { AnimatedContent, FadeContent } from "./BitsContent";
import { useMotionPreference } from "./motionPreferences";
import "./motion.css";

const effects = { enter: AnimatedContent, fade: FadeContent };

/** Portaled primitives are outside the provider's DOM, but share its policy. */
function PortalMotionPolicy() {
  const { policy } = useMotion();
  useLayoutEffect(() => {
    const root = document.documentElement;
    const previous = root.getAttribute("data-tron-motion-policy");
    root.setAttribute("data-tron-motion-policy", policy);
    return () => {
      if (previous === null) root.removeAttribute("data-tron-motion-policy");
      else root.setAttribute("data-tron-motion-policy", previous);
    };
  }, [policy]);
  return null;
}

export function TronMotionProvider({ children }: { children: ReactNode }) {
  const mode = useMotionPreference();
  const config = useMemo(() => ({ mode, duration: 180, distance: 8, transitionDuration: 120 }), [mode]);
  return <MotionProvider config={config} effects={effects}>
    <PortalMotionPolicy />
    {children}
  </MotionProvider>;
}
