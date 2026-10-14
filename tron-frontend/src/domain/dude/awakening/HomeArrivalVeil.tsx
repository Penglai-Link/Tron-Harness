import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useLocation } from "react-router";
import { CodeRain } from "./CodeRain";
import "./awakening.css";

type BackdropPhase = "hidden" | "rain" | "cover" | "reveal";
const ArrivalContext = createContext({
  showRain: (_visible: boolean) => {},
  beginArrival: (_reduced: boolean) => {},
});
export const useAwakeningBackdrop = () => useContext(ArrivalContext);

/** The same live canvas survives route changes; only its opacity changes. */
export function HomeArrivalVeil({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  const [phase, setPhase] = useState<BackdropPhase>("hidden");
  const [reduced, setReduced] = useState(false);
  const showRain = useCallback((visible: boolean) => {
    setPhase((current) => current === "cover" || current === "reveal"
      ? current : visible ? "rain" : "hidden");
  }, []);
  const beginArrival = useCallback((short: boolean) => {
    setReduced(short);
    setPhase("cover");
  }, []);
  useEffect(() => {
    if (pathname === "/awakening") return;
    if (phase === "rain") setPhase("hidden");
    if (phase !== "cover") return;
    // Wait until the destination has committed before unveiling it.
    let secondFrame = 0;
    const frame = requestAnimationFrame(() => {
      secondFrame = requestAnimationFrame(() => setPhase("reveal"));
    });
    return () => { cancelAnimationFrame(frame); cancelAnimationFrame(secondFrame); };
  }, [pathname, phase]);
  useEffect(() => {
    if (phase !== "reveal") return;
    const timer = setTimeout(() => setPhase("hidden"), reduced ? 240 : 3000);
    return () => clearTimeout(timer);
  }, [phase, reduced]);
  const actions = useMemo(() => ({ showRain, beginArrival }), [showRain, beginArrival]);
  return (
    <ArrivalContext.Provider value={actions}>
      {children}
      {phase !== "hidden" && (
        <div className="awakening-home-veil" data-phase={phase} data-destination={pathname !== "/awakening"} data-reduced={reduced} aria-hidden="true">
          <CodeRain departing={phase === "reveal"} />
        </div>
      )}
    </ArrivalContext.Provider>
  );
}
