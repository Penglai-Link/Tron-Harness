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
    children.set(parent, [...(children.get(parent) ?? []), span]);
  });
  const result: { span: TraceSpan; depth: number }[] = [];
  const seen = new Set<string>();
  const visit = (span: TraceSpan, depth: number) => {
    if (seen.has(span.spanId)) return;
    seen.add(span.spanId);
    result.push({ span, depth });
    (children.get(span.spanId) ?? []).forEach((child) =>
      visit(child, depth + 1)
    );
  };
  (children.get(null) ?? []).forEach((span) => visit(span, 0));
  spans.forEach((span) => visit(span, 0));
  return result;
}
