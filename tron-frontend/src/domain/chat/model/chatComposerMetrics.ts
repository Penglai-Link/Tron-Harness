import type { ChatAguiEvent } from "@tron/chat-core";
import type { ContextAssemblySnapshot } from "./contextAssemblyReport";

export interface ChatComposerMetrics {
  tokensPerSecond: number | null;
  cacheHitPercent: number | null;
  contextPercent: number | null;
  contextUsedTokens: number | null;
  contextWindowTokens: number | null;
  contextEstimated: boolean;
}

/** Legacy sessions may have a saved context report but no model audit events. */
export function contextMetricsFromSnapshot(snapshot: ContextAssemblySnapshot | null): ChatComposerMetrics | null {
  const used = tokenCount(snapshot?.estimatedInputTokens);
  // ContextOverviewPanel uses inputCapacity as its denominator. Match it so
  // the compact composer indicator stays numerically identical to the panel.
  const window = tokenCount(snapshot?.inputCapacity)
    ?? tokenCount(snapshot?.budgetBreakdown?.contextWindowLimit);
  if (used === null || !window) return null;
  return {
    tokensPerSecond: null,
    cacheHitPercent: null,
    contextPercent: Math.min(100, used / window * 100),
    contextUsedTokens: used,
    contextWindowTokens: window,
    contextEstimated: true,
  };
}

function sessionCacheHitPercent(events: ChatAguiEvent[]): number | null {
  const seen = new Set<string>();
  let inputTokens = 0;
  let cachedInputTokens = 0;
  let hasCacheUsage = false;

  for (const event of events) {
    if (event.eventName !== "agent.model.responded") continue;
    const audit = auditPayload(event);
    if (audit?.usageSource !== "provider" || audit.usageConfidence !== "actual") continue;
    const provider = record(audit.providerUsage);
    const input = tokenCount(provider?.inputTokens);
    const cached = tokenCount(provider?.cachedInputTokens);
    if (input === null) continue;

    const key = event.id || [event.runCode, event.occurredAt, audit.responseId ?? "", input,
      tokenCount(provider?.outputTokens) ?? "", tokenCount(provider?.totalTokens) ?? ""].join("\u0000");
    if (seen.has(key)) continue;
    seen.add(key);

    inputTokens += input;
    if (cached !== null) {
      cachedInputTokens += cached;
      hasCacheUsage = true;
    }
  }

  return inputTokens > 0 && hasCacheUsage
    ? Math.min(100, Math.round(cachedInputTokens / inputTokens * 100))
    : null;
}

/** Match dsh: sum paired provider output usage and decode time across the session. */
function sessionTokensPerSecond(events: ChatAguiEvent[]): number | null {
  const seen = new Set<string>();
  let outputTokens = 0;
  let decodeMs = 0;

  for (const event of events) {
    if (event.eventName !== "agent.model.responded") continue;
    const audit = auditPayload(event);
    if (audit?.usageSource !== "provider" || audit.usageConfidence !== "actual") continue;
    const output = tokenCount(record(audit.providerUsage)?.outputTokens);
    if (output === null) continue;

    let duration: number | null = null;
    try { duration = tokenCount(record(JSON.parse(event.metadata.model_timing_json ?? "null"))?.decodeMs); }
    catch { /* Older records have no reliable decode timing. */ }
    if (duration === null) continue;

    const attempt = event.metadata.model_attempt_id;
    const key = attempt
      ? [event.streamKey, event.runCode, "attempt", attempt].join("\u0000")
      : event.id || [event.streamKey, event.runCode, event.occurredAt, audit.responseId ?? "", output, duration].join("\u0000");
    if (seen.has(key)) continue;
    seen.add(key);

    // Keep numerator and denominator paired. Missing timings must not inflate
    // speed, and reasoning tokens are already included in provider output usage.
    outputTokens += output;
    decodeMs += duration;
  }

  const speed = decodeMs > 0 ? outputTokens * 1000 / decodeMs : null;
  return speed !== null && Number.isFinite(speed)
    ? speed >= 10 ? Math.round(speed) : Math.round(speed * 10) / 10 : null;
}

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown> : null;
}

function tokenCount(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : null;
}

function auditPayload(event: ChatAguiEvent): Record<string, unknown> | null {
  const encoded = event.metadata.usage_audit_json;
  if (!encoded) return null;
  try {
    return record(JSON.parse(encoded));
  } catch {
    return null;
  }
}

/** Session-wide generation speed and cache usage, with the latest context use. */
export function latestChatComposerMetrics(
  events: ChatAguiEvent[],
  contextSnapshot: ContextAssemblySnapshot | null = null,
): ChatComposerMetrics | null {
  const tokensPerSecond = sessionTokensPerSecond(events);
  const responses = events
    .filter((event) => event.eventName === "agent.model.responded")
    .sort((left, right) => Date.parse(right.occurredAt) - Date.parse(left.occurredAt));

  for (const response of responses) {
    const audit = auditPayload(response);
    if (!audit) continue;
    const estimate = record(audit.requestEstimate);
    const provider = audit.usageSource === "provider" && audit.usageConfidence === "actual"
      ? record(audit.providerUsage) : null;
    const inputTokens = tokenCount(provider?.inputTokens);
    const cachedInputTokens = tokenCount(provider?.cachedInputTokens);
    const contextWindowTokens = tokenCount(estimate?.effectiveInputBudget)
      ?? tokenCount(estimate?.contextWindowLimit);
    const estimatedInputTokens = tokenCount(estimate?.estimatedInputTokens);
    // The context inspector reports the assembled local estimate. Prefer its
    // persisted snapshot so the composer and inspector show the same number.
    const snapshotMetrics = contextMetricsFromSnapshot(contextSnapshot);
    const contextUsedTokens = snapshotMetrics?.contextUsedTokens ?? estimatedInputTokens ?? inputTokens;
    const contextWindow = snapshotMetrics?.contextWindowTokens ?? contextWindowTokens;
    const contextPercent = contextWindow && contextUsedTokens !== null
      ? Math.min(100, contextUsedTokens / contextWindow * 100) : null;
    const latestCacheHitPercent = inputTokens && cachedInputTokens !== null
      ? Math.min(100, Math.round(cachedInputTokens / inputTokens * 100)) : null;
    const cacheHitPercent = sessionCacheHitPercent(events) ?? latestCacheHitPercent;

    if (tokensPerSecond !== null || cacheHitPercent !== null || contextPercent !== null) {
      return {
        tokensPerSecond,
        cacheHitPercent,
        contextPercent,
        contextUsedTokens,
        contextWindowTokens: contextWindow,
        contextEstimated: snapshotMetrics !== null || estimatedInputTokens !== null,
      };
    }
  }
  return contextMetricsFromSnapshot(contextSnapshot);
}
