// Adapted from React Bits AnimatedContent / FadeContent. See NOTICE.md.
import { useLayoutEffect, useRef, type ReactNode } from "react";
import { gsap } from "gsap";
import type { ResolvedMotion } from "@personai/agent-ui-motion-react";

interface BitsContentProps {
  children?: ReactNode;
  motion: ResolvedMotion;
  replayKey?: string | number;
}

/** Animate the native child without introducing a layout or containing block. */
function BitsContent({ children, motion, replayKey }: BitsContentProps) {
  const root = useRef<HTMLDivElement>(null);
  const context = useRef<gsap.Context | null>(null);
  const previous = useRef<{ preset: string; replayKey?: string | number } | null>(null);

  useLayoutEffect(() => {
    const target = root.current?.firstElementChild;
    const changed = !previous.current || previous.current.preset !== motion.preset
      || previous.current.replayKey !== replayKey;
    previous.current = { preset: motion.preset, replayKey };

    if (!motion.enabled || motion.duration <= 0 || !target || target.closest("[hidden], [inert], [aria-hidden='true']")) {
      context.current?.revert();
      context.current = null;
      return;
    }
    // Configuration and streaming content updates never restart an entrance.
    if (!changed) return;
    context.current?.revert();
    const distance = motion.preset === "enter" ? motion.distance * motion.intensity : 0;
    context.current = gsap.context(() => {
      gsap.fromTo(target, { opacity: 0, y: distance }, {
        opacity: 1,
        y: 0,
        duration: motion.duration / 1000,
        delay: motion.delay / 1000,
        ease: ({ linear: "none", "ease-in": "power2.in", "ease-in-out": "power2.inOut" } as Record<string, string>)[motion.easing] ?? "power2.out",
        // Restore original inline styles, including transforms, after playback.
        onComplete: () => {
          context.current?.revert();
          context.current = null;
        },
      });
    }, root);
  });

  useLayoutEffect(() => () => {
    context.current?.revert();
    context.current = null;
    previous.current = null;
  }, []);

  return <div ref={root} data-tron-bits={motion.preset} style={{ display: "contents" }}>{children}</div>;
}

export function AnimatedContent(props: BitsContentProps) {
  return <BitsContent {...props} />;
}

export function FadeContent(props: BitsContentProps) {
  return <BitsContent {...props} />;
}
