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

/** A model call is matched by its attempt ID, so tool steps and other runs cannot skew tok/s. */
export function latestChatComposerMetrics(
  events: ChatAguiEvent[],
  contextSnapshot: ContextAssemblySnapshot | null = null,
): ChatComposerMetrics | null {
  const requests = new Map<string, ChatAguiEvent>();
  for (const event of events) {
    if (event.eventName === "agent.model.requested" && event.metadata.model_attempt_id) {
      requests.set(event.metadata.model_attempt_id, event);
    }
  }

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
    const outputTokens = tokenCount(provider?.outputTokens);
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

    const request = requests.get(response.metadata.model_attempt_id ?? "");
    const elapsedMs = request && request.runCode === response.runCode
      ? Date.parse(response.occurredAt) - Date.parse(request.occurredAt) : NaN;
    const tokensPerSecond = outputTokens !== null && Number.isFinite(elapsedMs) && elapsedMs > 0
      ? Math.round(outputTokens * 1000 / elapsedMs) : null;

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
