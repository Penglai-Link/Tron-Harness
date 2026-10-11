import type { TraceSpan } from "../model/Trace";
import { traceTree } from "./traceTree";

export function spanGraph(spans: TraceSpan[]) {
  const rows = traceTree(spans);
  const ancestors: string[] = [];
  const edges: { id: string; source: string; target: string }[] = [];
  for (const { span, depth } of rows) {
    if (depth > 0)
      edges.push({
        id: `parent:${span.spanId}`,
        source: ancestors[depth - 1],
        target: span.spanId,
      });
    ancestors[depth] = span.spanId;
    ancestors.length = depth + 1;
  }
  return { rows, edges };
}
