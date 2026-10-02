import { afterEach, describe, expect, it, vi } from "vitest";
import { createInternalChatAdapter } from "./index";

afterEach(() => vi.unstubAllGlobals());
describe("canonical replay health", () => {
  it("preserves known usage subtotal and marks its metadata incomplete", async () => {
    const event = { id: "1", type: "agent.model.usage", eventName: "agent.model.usage", runCode: "RUN", streamKey: "SESSION", occurredAt: "2026-10-02T00:00:00Z", outcome: "SUCCESS", metadata: { usage_audit_json: '{"actual":{"totalTokens":12}}' } };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ code: "SUCCESS", data: { events: [event], replayIncomplete: true } }), { headers: { "Content-Type": "application/json" } })));
    const events = await createInternalChatAdapter().loadAguiEvents("SESSION");
    expect(events).toEqual([{ ...event, metadata: { ...event.metadata, replayIncomplete: "true" } }]);
  });
  it("reports an incomplete empty replay through the existing transport error", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ code: "SUCCESS", data: { events: [], replayIncomplete: true } }), { headers: { "Content-Type": "application/json" } })));
    await expect(createInternalChatAdapter().loadAguiEvents("SESSION")).rejects.toMatchObject({ code: "AGENT_AGUI_REPLAY_INCOMPLETE", status: 503 });
  });
  it("accepts a complete empty replay", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ code: "SUCCESS", data: { events: [], replayIncomplete: false } }), { headers: { "Content-Type": "application/json" } })));
    expect(await createInternalChatAdapter().loadAguiEvents("SESSION")).toEqual([]);
  });
});
