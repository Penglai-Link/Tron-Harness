import { afterEach, describe, expect, it, vi } from "vitest";
import type { SessionHistory } from "@eliteforge/agent-ui-chat-core";
import { awaitAwakeningRun } from "./awaitAwakeningRun";

const empty = (): SessionHistory => ({
  sessionId: "guide", version: "0", nextCursor: null, runs: [], activeRuns: [], pendingInteractions: [],
  records: { messages: [], steps: [], toolInvocations: [], interactions: [] },
});
const run = { id: "wake", sessionId: "guide", parentRunId: null, rootRunId: "wake", status: "Running" as const, startedAtMs: 1, finishedAtMs: null, error: null };
afterEach(() => vi.useRealTimers());
describe("awakening queued-event handoff", () => {
  it("keeps waiting for an empty session, then hands off the real run", async () => {
    vi.useFakeTimers();
    const readHistory = vi.fn().mockResolvedValueOnce(empty()).mockResolvedValue({ ...empty(), activeRuns: [run] });
    const completed = vi.fn();
    const pending = awaitAwakeningRun({ sessionCode: "guide", readHistory, readEvents: async () => [], signal: new AbortController().signal, timeoutMessage: "timeout" }).then(completed);
    await vi.advanceTimersByTimeAsync(749);
    expect(completed).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    await pending;
    expect(readHistory).toHaveBeenCalledTimes(2);
    expect(completed).toHaveBeenCalledOnce();
  });
  it("waits past an old failed run while its retry is queued", async () => {
    vi.useFakeTimers();
    const readHistory = vi.fn().mockResolvedValueOnce({ ...empty(), runs: [{ ...run, status: "Failed", startedAtMs: 1 }] })
      .mockResolvedValue({ ...empty(), activeRuns: [{ ...run, startedAtMs: 20 }] });
    const completed = vi.fn();
    const pending = awaitAwakeningRun({ sessionCode: "guide", readHistory, readEvents: async () => [
      { eventId: "retry", sessionCode: "guide", source: "UserMessage", status: "Queued", createdAt: 10, result: null },
    ], signal: new AbortController().signal, timeoutMessage: "timeout" }).then(completed);
    await vi.advanceTimersByTimeAsync(749);
    expect(completed).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    await pending;
    expect(readHistory).toHaveBeenCalledTimes(2);
  });
  it("surfaces pre-run failures without confusing another session's events", async () => {
    const pending = awaitAwakeningRun({ sessionCode: "guide", readHistory: async () => empty(), readEvents: async () => [
      { eventId: "other", sessionCode: "private", source: "UserMessage", status: "Running", createdAt: 20, result: null },
      { eventId: "wake", sessionCode: "guide", source: "UserMessage", status: "Failed", createdAt: 10, result: "No model configured" },
    ], signal: new AbortController().signal, timeoutMessage: "timeout" });
    await expect(pending).rejects.toThrow("No model configured");
  });
  it("aborts pending reads and timers when leaving the page", async () => {
    vi.useFakeTimers();
    const controller = new AbortController();
    const readHistory = vi.fn().mockResolvedValue(empty());
    const pending = awaitAwakeningRun({ sessionCode: "guide", readHistory, readEvents: async () => [], signal: controller.signal, timeoutMessage: "timeout" });
    const rejected = expect(pending).rejects.toMatchObject({ name: "AbortError" });
    await vi.advanceTimersByTimeAsync(100);
    controller.abort();
    await rejected;
    await vi.advanceTimersByTimeAsync(60000);
    expect(readHistory).toHaveBeenCalledOnce();
    expect(vi.getTimerCount()).toBe(0);
  });
  it("reports a bounded timeout instead of leaving a permanent empty window", async () => {
    vi.useFakeTimers();
    const pending = awaitAwakeningRun({ sessionCode: "guide", readHistory: async () => empty(), readEvents: async () => [], signal: new AbortController().signal, timeoutMessage: "start timed out" });
    const rejected = expect(pending).rejects.toThrow("start timed out");
    await vi.advanceTimersByTimeAsync(45000);
    await rejected;
    expect(vi.getTimerCount()).toBe(0);
  });
});
