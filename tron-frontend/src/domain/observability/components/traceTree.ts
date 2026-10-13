import type { TraceSpan } from "../model/Trace";
export function traceTree(spans: TraceSpan[]) {
  const ids = new Set(spans.map((s) => s.spanId));
  const children = new Map<string | null, TraceSpan[]>();
  spans.forEach((span) => {
    const parent =
      span.parentSpanId &&
      ids.has(span.parentSpanId) &&
      span.parentSpanId !== span.spanId
        ? span.parentSpanId
        : null;
    const siblings = children.get(parent);
    if (siblings) siblings.push(span);
    else children.set(parent, [span]);
  });
  const result: { span: TraceSpan; depth: number }[] = [];
  const seen = new Set<string>();
  const visit = (root: TraceSpan) => {
    const stack = [{ span: root, depth: 0 }];
    while (stack.length) {
      const row = stack.pop()!;
      if (seen.has(row.span.spanId)) continue;
      seen.add(row.span.spanId);
      result.push(row);
      const descendants = children.get(row.span.spanId) ?? [];
      for (let i = descendants.length - 1; i >= 0; i--) stack.push({ span: descendants[i], depth: row.depth + 1 });
    }
  };
  (children.get(null) ?? []).forEach(visit);
  spans.forEach(visit);
  return result;
}
