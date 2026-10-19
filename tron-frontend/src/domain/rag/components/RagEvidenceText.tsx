import { useEffect, useMemo, useRef, useState } from "react";
import { layout, prepare } from "@chenglou/pretext";

/** Measured intrinsic height lets the browser skip offscreen source paragraphs. */
export function RagEvidenceText({ text }: { text: string }) {
  const element = useRef<HTMLParagraphElement>(null);
  const [metrics, setMetrics] = useState({ font: "", width: 0, lineHeight: 0, letterSpacing: 0 });
  const [fontRevision, setFontRevision] = useState(0);
  useEffect(() => {
    const node = element.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const style = getComputedStyle(node);
      const value = {
        font: `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`,
        width: entry.contentRect.width,
        lineHeight: Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.5,
        letterSpacing: Number.parseFloat(style.letterSpacing) || 0,
      };
      setMetrics(current => Object.entries(value).every(([key, field]) => current[key as keyof typeof current] === field) ? current : value);
    });
    observer.observe(node);
    const fontsChanged = () => setFontRevision(value => value + 1);
    document.fonts?.addEventListener("loadingdone", fontsChanged);
    return () => { observer.disconnect(); document.fonts?.removeEventListener("loadingdone", fontsChanged); };
  }, []);
  const prepared = useMemo(() => metrics.font ? prepare(text, metrics.font, { whiteSpace: "pre-wrap", letterSpacing: metrics.letterSpacing }) : null, [text, metrics.font, metrics.letterSpacing, fontRevision]);
  const height = useMemo(() => prepared && metrics.width > 0 ? layout(prepared, metrics.width, metrics.lineHeight).height : null, [prepared, metrics.width, metrics.lineHeight]);
  return <p ref={element} className="whitespace-pre-wrap break-words leading-relaxed" style={height === null ? undefined : { contentVisibility: "auto", containIntrinsicBlockSize: `auto ${height}px` }}>{text}</p>;
}
