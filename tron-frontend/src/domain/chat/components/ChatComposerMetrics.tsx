import { CircleGauge, Database, Gauge } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ChatComposerMetrics as Metrics } from "../model/chatComposerMetrics";
import "./ChatComposerMetrics.css";

export function ChatComposerMetrics({ metrics, onOpen }: { metrics: Metrics | null; onOpen: () => void }) {
  const metricsRef = useRef<HTMLDivElement>(null);
  const [hidden, setHidden] = useState(false);
  useLayoutEffect(() => {
    const element = metricsRef.current;
    const addon = element?.parentElement;
    const row = addon?.parentElement;
    if (!element || !addon || !row) return;

    const measure = () => {
      const gap = (node: Element) => parseFloat(getComputedStyle(node).columnGap) || 0;
      const siblings = Array.from(addon.children).filter((child) => child !== element && child instanceof HTMLElement && getComputedStyle(child).display !== "none") as HTMLElement[];
      const actions = Array.from(row.children).filter((child) => child !== addon && child instanceof HTMLElement && getComputedStyle(child).display !== "none") as HTMLElement[];
      const required = siblings.reduce((width, child) => width + child.getBoundingClientRect().width, 0)
        + actions.reduce((width, child) => width + child.getBoundingClientRect().width, 0)
        + element.scrollWidth + gap(addon) * siblings.length + gap(row) * actions.length;
      setHidden(required > row.clientWidth);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(row);
    observer.observe(element);
    Array.from(addon.children).forEach((child) => observer.observe(child));
    Array.from(row.children).forEach((child) => observer.observe(child));
    measure();
    return () => observer.disconnect();
  }, []);
  const { t, i18n } = useTranslation("pages");
  const number = new Intl.NumberFormat(i18n.language);
  const contextNumber = new Intl.NumberFormat(i18n.language, {
    maximumFractionDigits: metrics?.contextPercent != null && metrics.contextPercent < 10 ? 1 : 0,
  });
  const unavailable = t("chat.composerMetrics.unavailable");
  return <div ref={metricsRef} className="tron-chat-composer-metrics" data-hidden={hidden || undefined} aria-hidden={hidden || undefined} inert={hidden} role="group" aria-label={t("chat.composerMetrics.ariaLabel")}>
    <button className="tron-chat-composer-metrics__item" title={t("chat.composerMetrics.throughputTitle")} type="button" onClick={onOpen}>
      <Gauge size={14} aria-hidden="true" />
      {t("chat.composerMetrics.throughput", { value: metrics?.tokensPerSecond == null ? t("chat.composerMetrics.speedUnavailable") : number.format(metrics.tokensPerSecond) })}
    </button>
    <button className="tron-chat-composer-metrics__item" title={t("chat.composerMetrics.cacheHitTitle")} type="button" onClick={onOpen}>
      <Database size={14} aria-hidden="true" />
      {t("chat.composerMetrics.cacheHit", { value: metrics?.cacheHitPercent == null ? unavailable : number.format(metrics.cacheHitPercent) })}
    </button>
    <button className="tron-chat-composer-metrics__item" title={metrics?.contextPercent != null && metrics.contextUsedTokens != null && metrics.contextWindowTokens != null
      ? t("chat.composerMetrics.contextTitle", {
        used: number.format(metrics.contextUsedTokens), limit: number.format(metrics.contextWindowTokens),
        source: metrics.contextEstimated ? t("chat.composerMetrics.estimated") : t("chat.composerMetrics.measured"),
      }) : t("chat.composerMetrics.contextUnavailableTitle")} type="button" onClick={onOpen}>
      <CircleGauge size={14} aria-hidden="true" />
      {t("chat.composerMetrics.context", { value: metrics?.contextPercent == null ? unavailable : contextNumber.format(metrics.contextPercent) })}
    </button>
  </div>;
}
