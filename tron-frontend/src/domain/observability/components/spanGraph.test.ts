import { expect, it } from "vitest";
import { spanGraph } from "./spanGraph";
import type { TraceSpan } from "../model/Trace";
const span = (spanId: string, parentSpanId: string | null) => ({ spanId, parentSpanId } as TraceSpan);
it("keeps parent-child calls and isolates missing parents without producing cyclic topology", () => {
  const graph = spanGraph([span("child", "root"), span("root", null), span("orphan", "missing"), span("a", "b"), span("b", "a")]);
  expect(graph.edges).toEqual([{ id: "parent:child", source: "root", target: "child" }, { id: "parent:b", source: "a", target: "b" }]);
  expect(graph.rows).toHaveLength(5);
});
