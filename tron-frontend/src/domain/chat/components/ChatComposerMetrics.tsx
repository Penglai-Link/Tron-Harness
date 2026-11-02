import { CircleGauge, Database, Gauge } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ChatComposerMetrics as Metrics } from "../model/chatComposerMetrics";
import "./ChatComposerMetrics.css";

export function ChatComposerMetrics({ metrics, onOpen }: { metrics: Metrics | null; onOpen: () => void }) {
  const { t, i18n } = useTranslation("pages");
  const number = new Intl.NumberFormat(i18n.language);
  const contextNumber = new Intl.NumberFormat(i18n.language, {
    maximumFractionDigits: metrics?.contextPercent != null && metrics.contextPercent < 10 ? 1 : 0,
  });
  const unavailable = t("chat.composerMetrics.unavailable");
  return <div className="tron-chat-composer-metrics" role="group" aria-label={t("chat.composerMetrics.ariaLabel")}>
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
