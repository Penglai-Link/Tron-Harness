import { useEffect, useMemo, useRef, useState } from "react";
import { layout, prepare } from "@chenglou/pretext";

/** Pretext estimates long plain-text height without forcing paragraph reflows. */
export function NotificationText({ text }: { text: string }) {
  const ref = useRef<HTMLParagraphElement>(null);
  const [metrics, setMetrics] = useState({ font: "", width: 0, lineHeight: 0, letterSpacing: 0 });
  const [fontRevision, setFontRevision] = useState(0);
  useEffect(() => {
    const node = ref.current;
    if (!node || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      const style = getComputedStyle(node);
      const next = {
        font: `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`,
        width: node.clientWidth,
        lineHeight: Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.75,
        letterSpacing: Number.parseFloat(style.letterSpacing) || 0,
      };
      setMetrics((current) => Object.entries(next).every(([key, value]) => current[key as keyof typeof current] === value) ? current : next);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    const fontsChanged = () => { measure(); setFontRevision((value) => value + 1); };
    document.fonts?.addEventListener("loadingdone", fontsChanged);
    return () => { observer.disconnect(); document.fonts?.removeEventListener("loadingdone", fontsChanged); };
  }, []);
  const measurable = metrics.width > 0;
  const prepared = useMemo(() => metrics.font && measurable
    ? prepare(text, metrics.font, { whiteSpace: "pre-wrap", letterSpacing: metrics.letterSpacing }) : null,
  [text, metrics.font, measurable, metrics.letterSpacing, fontRevision]);
  const height = useMemo(() => prepared && metrics.width > 0
    ? layout(prepared, metrics.width, metrics.lineHeight).height : null, [prepared, metrics.width, metrics.lineHeight]);
  return <p ref={ref} className="whitespace-pre-wrap text-sm leading-7 [overflow-wrap:anywhere]"
    style={height === null ? undefined : { contentVisibility: "auto", containIntrinsicBlockSize: `auto ${height}px` }}>{text}</p>;
}
