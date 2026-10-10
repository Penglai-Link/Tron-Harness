import { describe, expect, it } from "vitest";
import { traceTree } from "./traceTree";
import type { TraceSpan } from "../model/Trace";
const span = (spanId: string, parentSpanId: string | null): TraceSpan => ({
  spanId,
  parentSpanId,
  name: "agent.model",
  startedAt: 0,
  durationMs: 1,
  status: "Completed",
  attributes: {},
  events: [],
  links: [],
});
describe("traceTree", () => {
  it("keeps partial traces navigable and emits cyclic spans once", () => {
    const rows = traceTree([
      span("child", "root"),
      span("root", null),
      span("orphan", "missing"),
      span("a", "b"),
      span("b", "a"),
    ]);
    expect(
      rows.slice(0, 3).map(({ span: s, depth }) => [s.spanId, depth])
    ).toEqual([
      ["root", 0],
      ["child", 1],
      ["orphan", 0],
    ]);
    expect(new Set(rows.map(({ span: s }) => s.spanId)).size).toBe(5);
    expect(rows).toHaveLength(5);
  });
});
