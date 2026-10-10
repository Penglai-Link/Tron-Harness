// React Bits ShinyText gradient sweep, adapted for Tron theme and motion policy.
// See NOTICE.md. CSS playback avoids one JavaScript animation loop per label.
import type { HTMLAttributes } from "react";
import { useMotion } from "@eliteforge/agent-ui-motion-react";
import { cn } from "@/shared/lib/utils";
import "./motion.css";

export function ShinyText({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  const { policy } = useMotion();
  return <span {...props} data-tron-shiny-text="" data-motion={policy}
    className={cn("tron-shiny-text", className)} />;
}
