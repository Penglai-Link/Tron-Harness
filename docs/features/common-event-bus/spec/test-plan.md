# Test Plan: Common Event Bus

## Test Plan Decision

- status: draft_pending_confirmation
- approved_by: pending
- approved_at: pending

Valid statuses: `draft_pending_confirmation`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-CEB-01 | layer: backend | case: shared contract boundary | red: business modules publish through sink-specific code or no shared module exists | green: `shared/event_bus` exposes the common publisher, event, and sink contracts and business services depend only on that boundary | command_ref: CMD-BE-SEARCH.
- AC-CEB-02 | layer: backend | case: bounded envelope validation | red: event structs allow unbounded maps, oversize payloads, or uncontrolled string growth | green: draft validation rejects oversize attrs and payload while keeping a normalized fixed-shape envelope | command_ref: CMD-BE-UNIT.
- AC-CEB-03 | layer: backend | case: non-blocking publish path | red: publication awaits sink work or uses unbounded queues | green: admission uses bounded queues and returns explicit outcomes without blocking the request path | command_ref: CMD-BE-UNIT.
- AC-CEB-04 | layer: backend | case: lane isolation under pressure | red: AGUI or observability floods can fill the only ingress lane and starve critical audit or security signals | green: critical lane retains reserved capacity and tests prove bulk flood does not consume it | command_ref: CMD-BE-UNIT.
- AC-CEB-05 | layer: backend | case: AGUI replay budget and eviction | red: disconnected AGUI streams keep replay forever or global replay memory grows without a hard stop | green: replay exists only for active streams, obeys per-stream caps, and is evicted by TTL or global budget pressure | command_ref: CMD-BE-UNIT.
- AC-CEB-06 | layer: backend/database | case: audit sink independence | red: producers know about SQLite rows or audit persistence is fused into every service path | green: audit persistence is reachable only through a sink boundary and can be added in batch form without changing producers | command_ref: CMD-BE-SEARCH.
- AC-CEB-07 | layer: backend | case: runtime observability of failures | red: invalid, oversize, full-queue, and sink-lag events disappear silently | green: stats expose accepted, rejected, dropped, and evicted counts | command_ref: CMD-BE-UNIT.
- AC-CEB-08 | layer: backend/security | case: secret and snapshot guardrails | red: services can publish plaintext passwords, raw tokens, full request bodies, or giant snapshots | green: tests and contract checks reject forbidden payload classes or oversize content | command_ref: CMD-BE-UNIT.
- AC-CEB-09 | layer: integration | case: AppState injection and publish-after-commit semantics | red: current service construction cannot adopt the publisher without invasive rewrites or publishes while repository locks are held | green: `AppState` builds one runtime, injects a cloneable handle, and services publish after successful commit points | command_ref: CMD-BE-CHECK.
- AC-CEB-10 | layer: backend | case: deterministic testing | red: queue-pressure or replay tests require wall-clock sleeps, real network sinks, or nondeterministic IDs | green: fixed IDs, test clocks, and in-memory test sinks make RED and GREEN evidence stable | command_ref: CMD-BE-UNIT.

## Command Index

- CMD-BE-SEARCH | command: `rtk proxy rg "shared/event_bus|EventBusHandle|BusEvent|EventSink|Agui" tron-backend/src` | cost: low | when: review | rerun: when shared-event-bus paths change | expect: shared event-bus boundary exists and sink-specific details stay outside producers.
- CMD-BE-UNIT | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml event_bus` | cost: medium | when: red-green-verify | rerun: after shared-event-bus code changes | expect: queue pressure, validation, replay eviction, and publish contract tests fail in RED and pass in GREEN.
- CMD-BE-CHECK | command: `rtk cargo check --manifest-path tron-backend/Cargo.toml` | cost: medium | when: verify | rerun: after backend wiring changes | expect: AppState and service wiring compile with injected publisher handles.
- CMD-BE-FMT | command: `rtk cargo fmt --manifest-path tron-backend/Cargo.toml --check` | cost: low | when: verify | rerun: after Rust source edits | expect: shared-event-bus source matches repo formatting rules.

## Evidence Rules

- RED evidence must prove one missing contract or one failing capacity guard at a time.
- GREEN evidence must prove the same contract after implementation without broadening memory caps silently.
- Queue-full behavior must be asserted by outcome and counters, not inferred from logs alone.
- AGUI replay evidence must include both per-stream and global-budget eviction behavior.
- No browser evidence is required for the shared module itself.

## Regression Scope

- Re-run shared-event-bus unit tests after any change to publish validation, queue budgets, or AGUI replay rules.
- Re-run compile verification after `AppState`, service constructors, or shared exports change.
- Review all producer paths for accidental secret publication whenever auth, password reset, or AGUI payload mappers change.
