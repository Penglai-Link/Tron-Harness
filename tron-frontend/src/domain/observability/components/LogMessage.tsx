import { LogHighlight } from "./LogHighlight";
import { useEffect, useMemo, useRef, useState } from "react";
import { prepare, layout } from "@chenglou/pretext";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

/** Measure bounded log text once, then reuse Pretext layout when its column resizes. */
export function LogMessage({ message, continuous = false, keyword = "" }: { message: string; continuous?: boolean; keyword?: string }) {
  const { t } = useTranslation("observability");
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(320);
  const [font, setFont] = useState("12px monospace");
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const update = () => {
      setWidth(element.clientWidth || 320);
      setFont(getComputedStyle(element).font || "12px monospace");
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const prepared = useMemo(
    () => prepare(message, font, { whiteSpace: "pre-wrap" }),
    [message, font]
  );
  const long = layout(prepared, width, 20).lineCount > 2;
  return (
    <div ref={ref} className={`min-w-0 font-mono text-xs leading-5 ${continuous ? "" : "max-w-xl"}`}>
      <div
        className={`whitespace-pre-wrap break-all ${
          expanded || continuous ? "" : "line-clamp-2"
        }`}
      >
        <LogHighlight text={message || "—"} keyword={keyword} />
      </div>
      {long && !continuous && (
        <Button
          variant="link"
          className="h-auto p-0 text-xs"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
        >
          {t(expanded ? "showLess" : "showMore")}
        </Button>
      )}
    </div>
  );
}
