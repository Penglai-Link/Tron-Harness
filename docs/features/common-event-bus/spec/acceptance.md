# Acceptance: Common Event Bus

## Acceptance Index

- AC-CEB-01 [must] [backend] Tron provides a shared `tron-backend/src/shared/event_bus` module with Tron-owned publisher, event, and sink contracts so business modules do not couple directly to audit, observability, or AGUI implementations; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-02 [must] [backend] One normalized event envelope covers audit, observability, and AGUI reuse through fixed header fields plus bounded optional attributes and payload, rather than unbounded free-form maps; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-03 [must] [performance] Event publication is non-blocking and fully bounded: the request or mutation path does not await sink I/O, use unbounded queues, or perform large payload serialization before admission; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-04 [must] [capacity] Critical audit or security events retain reserved admission capacity even when bulk AGUI or observability traffic is noisy, so one high-volume class cannot starve the others; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-05 [must] [agui] AGUI reuse works through stream-scoped fan-out with bounded replay, TTL eviction, and a hard global memory cap; the base runtime keeps no global replay history; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-06 [must] [audit] Audit reuse is enabled through a sink boundary that can later persist selected events in batch without changing producer contracts or forcing every event class into storage; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-07 [must] [observability] Queue overflow, oversize rejection, sink lag, filter skips, and replay eviction are observable through explicit counters or runtime stats rather than disappearing silently; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-08 [must] [security] The shared contract forbids plaintext passwords, secrets, full request bodies, and large entity snapshots on the hot path; only bounded summaries and references may be published; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-09 [should] [integration] `AppState` and current backend services can adopt the shared publisher with limited refactor by injecting a cloneable handle and publishing after commit points; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CEB-10 [should] [testability] The shared module supports deterministic tests for queue pressure, stream eviction, validation rejection, and producer contracts without wall-clock sleeps or real external sinks; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
