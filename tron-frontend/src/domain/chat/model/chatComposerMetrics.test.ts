import { describe, expect, it } from "vitest";
import type { ChatAguiEvent } from "@tron/chat-core";
import { contextMetricsFromSnapshot, latestChatComposerMetrics } from "./chatComposerMetrics";

function event(eventName: string, occurredAt: string, metadata: Record<string, string>, runCode = "RUN"): ChatAguiEvent {
  return { id: `${eventName}-${occurredAt}`, type: eventName, eventName, runCode,
    streamKey: "SESSION", occurredAt, outcome: "COMPLETED", metadata };
}

function audit(inputTokens: number | null, outputTokens: number | null, cachedInputTokens: number | null) {
  return JSON.stringify({
    usageSource: inputTokens === null ? "unavailable" : "provider",
    usageConfidence: inputTokens === null ? "unavailable" : "actual",
    providerUsage: inputTokens === null ? null : { inputTokens, outputTokens, cachedInputTokens },
    requestEstimate: { estimatedInputTokens: 40_000, contextWindowLimit: 100_000 },
  });
}

describe("latestChatComposerMetrics", () => {
  it("uses the matching model attempt and provider usage for all three indicators", () => {
    const events = [
      event("agent.model.requested", "2026-10-19T12:00:00.000Z", { model_attempt_id: "A" }),
      event("agent.model.requested", "2026-10-19T12:00:01.000Z", { model_attempt_id: "B" }),
      event("agent.model.responded", "2026-10-19T12:00:02.000Z", { model_attempt_id: "A", usage_audit_json: audit(30_000, 200, 9_000) }),
    ];
    expect(latestChatComposerMetrics(events)).toEqual({
      tokensPerSecond: 100, cacheHitPercent: 30, contextPercent: 40,
      contextUsedTokens: 40_000, contextWindowTokens: 100_000, contextEstimated: true,
    });
  });

  it("keeps unknown provider metrics absent and labels context estimates", () => {
    const events = [event("agent.model.responded", "2026-10-19T12:00:02.000Z", {
      model_attempt_id: "A", usage_audit_json: audit(null, null, null),
    })];
    expect(latestChatComposerMetrics(events)).toEqual({
      tokensPerSecond: null, cacheHitPercent: null, contextPercent: 40,
      contextUsedTokens: 40_000, contextWindowTokens: 100_000, contextEstimated: true,
    });
    expect(latestChatComposerMetrics([event("agent.model.responded", "2026-10-19T12:00:02.000Z", {
      usage_audit_json: "invalid",
    })])).toBeNull();
  });

  it("uses the latest response and never pairs requests from another run", () => {
    const events = [
      event("agent.model.requested", "2026-10-19T12:00:00.000Z", { model_attempt_id: "A" }, "OLD"),
      event("agent.model.responded", "2026-10-19T12:00:02.000Z", { model_attempt_id: "A", usage_audit_json: audit(30_000, 200, 9_000) }, "OLD"),
      event("agent.model.responded", "2026-10-19T12:01:02.000Z", { model_attempt_id: "A", usage_audit_json: audit(10_000, 50, 0) }, "NEW"),
    ];
    expect(latestChatComposerMetrics(events)).toMatchObject({
      tokensPerSecond: null, cacheHitPercent: 23, contextPercent: 40,
    });
  });

  it("uses cumulative cache usage while keeping context aligned with the assembly snapshot", () => {
    const events = [
      event("agent.model.responded", "2026-10-19T12:00:02.000Z", { model_attempt_id: "A", usage_audit_json: audit(30_000, 200, 9_000) }),
      event("agent.model.responded", "2026-10-19T12:01:02.000Z", { model_attempt_id: "B", usage_audit_json: audit(10_000, 50, 0) }),
    ];
    const snapshot = {
      blocks: [], protocolTokens: null, inputCapacity: 1_290_000, modelCode: null,
      estimatedInputTokens: 33_000, stage: null, step: null, attempt: null,
      budgetBreakdown: { contextWindowLimit: 1_300_000 },
    };
    expect(latestChatComposerMetrics(events, snapshot)).toMatchObject({
      cacheHitPercent: 23, contextPercent: 33_000 / 1_290_000 * 100, contextUsedTokens: 33_000,
      contextWindowTokens: 1_290_000, contextEstimated: true,
    });
  });
});

it("restores context use from a saved report without inventing legacy speed or cache values", () => {
  expect(contextMetricsFromSnapshot({
    blocks: [], protocolTokens: null, inputCapacity: 1_294_336, modelCode: null,
    estimatedInputTokens: 58_208, stage: null, step: null, attempt: null,
    budgetBreakdown: { contextWindowLimit: 1_310_720 },
  })).toEqual({
    tokensPerSecond: null, cacheHitPercent: null, contextPercent: 58_208 / 1_294_336 * 100,
    contextUsedTokens: 58_208, contextWindowTokens: 1_294_336, contextEstimated: true,
  });
});
