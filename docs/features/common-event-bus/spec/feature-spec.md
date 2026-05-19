# Feature Spec: Common Event Bus

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: draft_pending_confirmation

## Feature Identity And Context

- feature_slug: common-event-bus
- feature_name: Common Event Bus
- status: draft
- owner: clouds3n
- source_request: design a backend shared bus module that unifies internal event handling for audit, observability, and future AGUI reuse without wasting memory
- main_agent: lead agent on the active Tron checkout
- dependency_features: `common-snowflake-id`, `auth`, `user-management-model`, `workspace-management`, `task-management`
- dependency_contracts: `tron-backend/src/app.rs` service wiring, `tron-backend/src/shared/id` common capability, Axum request/auth context, and SQLite-backed repositories

## Requirement

Tron needs a backend shared event-bus module that gives every internal module one consistent way to publish mutation, security, operational, and realtime session signals. The same event contract must support later audit persistence, observability sinks, and AGUI session streams without turning every feature into its own logging pipeline or leaving unbounded buffers in memory.

## Scope Boundary

This slice owns the in-process backend event contract, publish and dispatch runtime, sink abstraction, capacity controls, memory budgets, and future-compatible boundaries for audit and AGUI consumers. It does not define a public UI, a distributed broker, or a compliance-grade immutable audit product by itself.

## Assumptions

- Tron currently runs as a Rust monolith, so an in-process bus is the right first boundary.
- Business services can publish events after successful persistence or lifecycle transitions instead of refactoring into full event sourcing.
- Later audit durability may require a transactional outbox or dedicated persistence sink; the common bus should standardize event shape and routing first.
- Later AGUI consumers need short-lived realtime or near-realtime stream fan-out, not global long-term replay.
- The existing `common-snowflake-id` capability can supply event IDs when deterministic test IDs are not injected.
- Most event names are compile-time constants, so the hot path should prefer enums or `&'static str` over dynamic strings whenever possible.

## In Scope

- A Tron-owned shared module under `tron-backend/src/shared/event_bus`.
- A normalized event envelope that can be reused by audit, observability, and AGUI consumers.
- Publish-side validation and size guards that reject oversize or invalid events before they enter queues.
- Bounded ingress lanes and bounded sink queues with explicit drop or reject outcomes.
- Reserved capacity so noisy AGUI or observability traffic cannot starve critical audit or security signals.
- Sink abstractions for audit persistence, observability export, and AGUI stream fan-out.
- Runtime counters for accepted, dropped, rejected, oversized, and filtered events.
- App bootstrapping and service injection rules so business modules depend only on the shared publisher contract.
- Future database design for an optional audit outbox sink.

## Out Of Scope

- Kafka, NATS, Redis Streams, or any cross-process event transport.
- Exactly-once delivery, global ordering across processes, or distributed replay semantics.
- A permanent event history for every event class.
- Storing large request or response bodies, entity snapshots, plaintext secrets, or arbitrary debug dumps in the hot path.
- Public SSE or WebSocket endpoints; AGUI transport adapters can be added later on top of the shared sink contract.
- Requiring read-only query paths to publish high-volume low-value events by default.

## Constraints

- No unbounded queue, map, replay cache, or subscriber registry is allowed.
- The request and mutation hot path must not await sink I/O.
- The shared module must minimize allocations by keeping the envelope fixed-shape and bounded.
- Event publication must happen after repository lock release and after the mutation result is known.
- Producers must not depend on SQLite schema details, AGUI transport details, or observability vendor APIs.
- The shared module must be testable with deterministic queues, clocks, and IDs.

## Feature Model

### Actors

- Backend producer: a business service, middleware layer, or runtime component that emits an internal event.
- Event bus runtime: validates, admits, and dispatches accepted events.
- Audit sink: persists selected high-value events for later audit or security review.
- Observability sink: converts selected events into counters, logs, traces, or health signals.
- AGUI sink: fans out selected stream-scoped events to active runtime viewers with bounded replay.

### Core Objects

- `BusEventId`: Snowflake-backed internal event identifier.
- `BusRouteClass`: `Audit`, `Observe`, or `Agui`.
- `BusPriority`: `Critical`, `Standard`, or `Debug`.
- `BusEvent`: immutable normalized envelope published on the bus.
- `BusSubjectRef`: stable subject reference such as `userCode`, `workspaceCode`, or `taskCode`.
- `BusActorRef`: optional actor context such as `userCode`, auth provider, or system actor.
- `BusRequestRef`: optional request context such as `requestId`, `traceId`, route, and session key.
- `BusEventAttr`: bounded key/value metadata entry for compact extra facts.
- `EventBusHandle`: cloneable publisher handle injected into services and runtime components.
- `EventSink`: sink contract that receives `Arc<BusEvent>` through its own bounded queue.
- `BusRuntimeStats`: counters and gauges for queue pressure and drop reasons.
- `AguiStreamRegistry`: dynamic but bounded stream watcher registry owned only by the AGUI sink.

### Event Classes

| route class | primary purpose | typical producers | persistence default |
| --- | --- | --- | --- |
| `Audit` | security-sensitive and business-significant lifecycle changes | auth, user, workspace, task services | persistable |
| `Observe` | operational or diagnostic signals such as failures, queue pressure, integration latency, and retry outcomes | adapters, middleware, jobs | ephemeral |
| `Agui` | agent-run, task-run, or session-run progress that a live AGUI surface may stream | agent runtime, task runtime, orchestrators | ephemeral |

### Rules

- Every event must have a fixed header: `event_id`, `event_name`, `route_class`, `priority`, `producer`, `occurred_at_ms`, and `outcome`.
- `event_name`, `producer`, and `subject_type` should be enums or `&'static str` constants on the producer side; dynamic strings are reserved for business codes and bounded display summaries.
- Optional metadata is bounded: `max_attrs = 8`, `max_attr_key_bytes = 32`, `max_attr_value_bytes = 96`.
- Optional payload is bounded and off by default: `max_payload_bytes = 1024`. Large tool output, request bodies, or entity snapshots must not pass through the bus.
- The bus uses two ingress lanes by default:
  - `critical_ingress_capacity = 128`
  - `standard_ingress_capacity = 512`
- Each sink has its own bounded queue. Default `sink_queue_capacity = 256`, overridable per sink.
- Publication uses `try_send` semantics. Queue pressure returns a visible `PublishOutcome`; it must not block the request path.
- The dispatcher forwards `Arc<BusEvent>` to sinks so one event allocation is shared across fan-out.
- No global event retention exists in the base runtime.
- AGUI replay is stream-scoped only:
  - buffer created only when at least one watcher is active
  - `agui_replay_events_per_stream = 32`
  - `agui_replay_bytes_per_stream = 64 KiB`
  - `agui_global_replay_bytes = 2 MiB`
  - `agui_stream_idle_ttl_seconds = 30`
- Critical audit or security events must not share their only capacity lane with bulk AGUI chatter.
- Queue overflow, invalid payload size, attribute overflow, sink lag, and AGUI replay eviction must all be counted.

### Capacity And Memory Budget

- Base ingress budget is intentionally small and explicit: 640 queued events across both lanes.
- The hot-path envelope stays compact by preferring fixed enums or `&'static str` and a bounded attribute list rather than open-ended `serde_json::Value` maps.
- AGUI replay has a hard global cap and no idle-stream retention after TTL expiry.
- The base module owns no event history cache, no global replay log, and no per-producer backlog.
- The default design favors dropping low-value overflow over growing memory invisibly. If a future audit requirement cannot tolerate loss, that requirement must move to a transactional outbox path instead of weakening the whole runtime with unbounded retention.

### State Transitions

- producer builds `BusEventDraft` -> validation normalizes bounded envelope -> event accepted into `critical` or `standard` lane
- event admitted -> dispatcher fans out `Arc<BusEvent>` to registered sinks
- sink accepts -> sink-local async worker flushes, exports, or broadcasts
- sink queue full or filter reject -> sink-local drop outcome is counted without blocking producers
- AGUI watcher connected -> per-stream bounded replay buffer becomes active
- AGUI watcher disconnected -> stream buffer expires after idle TTL or earlier under global budget pressure
- future audit sink enabled -> selected `Audit` events are batch-persisted without changing producer contracts

## Module And Boundary Map

### Backend Modules To Add

- `tron-backend/src/shared/event_bus/mod.rs`
- `tron-backend/src/shared/event_bus/config.rs`
- `tron-backend/src/shared/event_bus/event.rs`
- `tron-backend/src/shared/event_bus/runtime.rs`
- `tron-backend/src/shared/event_bus/sink.rs`
- `tron-backend/src/shared/event_bus/testing.rs`

### Supporting Integration Points

- `tron-backend/src/shared.rs`: export `event_bus`
- `tron-backend/src/config.rs`: add bounded event-bus runtime configuration
- `tron-backend/src/app.rs`: build the bus runtime once and inject `EventBusHandle` into services
- `tron-backend/src/application/user.rs` or `.../user/service.rs`: publish user lifecycle and auth-significant events after successful mutations
- `tron-backend/src/application/workspace.rs`: publish workspace create, move, archive, restore, and access-grant replacement events
- `tron-backend/src/application/task.rs`: publish task create, update, status, archive, and restore events
- `tron-backend/src/application/role.rs`: publish role-catalog replacement events
- future adapters:
  - `tron-backend/src/adapters/outbound/persistence/sqlite/audit_event_outbox.rs`
  - `tron-backend/src/adapters/outbound/observability/event_metrics_sink.rs`
  - `tron-backend/src/adapters/outbound/agui/agui_stream_sink.rs`

### Integration Policy

- Producers depend only on `EventBusHandle` and shared event types.
- Producers publish after the business mutation has committed and after repository locks are released.
- Handlers or middleware may attach request context, but they must not own sink-specific serialization logic.
- The audit sink decides what persists.
- The AGUI sink decides what stream keys are visible and how replay buffers are budgeted.

## Repository Findings

- `tron-backend/src/shared/id` already demonstrates the intended pattern for a backend shared capability with Tron-owned contracts and test adapters.
- `tron-backend/src/app.rs` already centralizes service construction, which is the right place to build one event-bus runtime and clone publisher handles into services.
- Current services use `Arc<Mutex<Box<dyn Repository>>>`, so event publication must occur after lock release to avoid widening contention.
- The backend already has a point solution for password reset events in SQLite, but no unified event contract or dispatcher exists yet.
- Tokio is already present in the backend, so a bounded async runtime with `mpsc` fan-out fits the current stack without introducing a heavier dependency.
- No current backend module appears to provide AGUI-specific stream infrastructure, which makes a neutral shared event contract more important than a sink-specific design.

## Open Decisions

- None. The opinionated default is: shared in-process bus first, bounded queues everywhere, audit durability later through a dedicated sink rather than through unbounded in-memory retention.
