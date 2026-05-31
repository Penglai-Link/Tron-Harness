# Feature Evolution Log: RAG 后端底座 - 诊断、重试、Trace 与事件

## Summary

- feature_slug: rag-diagnostics-retry-trace-events
- source_prd: `docs/brainstorm/rag-backend-foundation/prd.typ`
- status: new_feature

## Change History

| date | change | reason |
| --- | --- | --- |
| 2026-05-31 | created feature workspace for diagnostics/retry/trace/events | PRD recommended order reached final first-stage RAG backend slice |

## Downstream Artifact Migration

- No stale artifacts at creation.

## Downstream Impact

- Updates previous search boundary by allowing trace persistence only under this feature.
- Updates previous RAG boundary by allowing debug/retry routes only under this feature.
- Keeps GraphService、SummaryTree、code-graph and Memory governance out of scope.

## Residual Risks

- EventBus publish failure handling must remain non-blocking.
- Search trace may grow quickly if enabled broadly; TTL cleanup must be covered by repository behavior or cleanup method.
