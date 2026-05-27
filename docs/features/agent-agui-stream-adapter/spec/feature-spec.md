# Feature Spec: Agent AG-UI Stream Adapter

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: agent-agui-stream-adapter
- feature_name: Agent AG-UI Stream Adapter
- status: approved_with_defaults
- owner: clouds3n
- source_request: complete feature 4 from `docs/brainstorm/agent-framework/prd.typ`
- main_agent: Codex lead agent on `feature/clouds3n/agent-framework/base`
- dependency_features: `react-agent-runtime`, `common-event-bus`
- dependency_contracts: `BusEvent`, `AguiEventSink`, `EventBusHandle`, runtime stream key, protected auth route

## Requirement

Expose a protected backend adapter that activates an AG-UI stream, consumes replayable unified agent events, converts them into safe AG-UI-shaped events, and lets consumers retrieve stream snapshots without exposing raw prompts, completions, tool outputs, API keys, or request bodies.

## Product Source Slices

- prd_feature_headings: `Feature Inventory` row `Agent AG-UI Stream Adapter`
- prd_rule_headings: event filtering and protocol conversion from the feature inventory
- prd_exception_headings: disconnected consumer and stale stream from the feature inventory
- dependency_order: follows Feature 1 and common event bus

## Scope Boundary

This feature adds backend AG-UI stream activation and replay conversion. It does not add frontend AG-UI widgets, persisted history, or a second event bus.

## Assumptions

- Existing `AguiEventSink` stores replay only for activated stream keys.
- Runtime events already publish metadata-only `BusEventDraft::agui` events with stream keys.
- The first adapter returns replay snapshots as JSON; live SSE can evolve later without changing event conversion semantics.

## In Scope

- Protected stream activation endpoint.
- Protected replay endpoint that converts stored bus events to AG-UI-shaped event objects.
- Protected stream deactivation endpoint.
- Filtering to agent events for a requested stream key.
- Response safety checks.
- Backend tests proving activation, invocation capture, conversion, and redaction.

## Out Of Scope

- Frontend AG-UI consumer component.
- SSE transport.
- Persisted run history; feature3 owns history.
- Creating a separate agent event bus.

## Constraints

- Adapter consumes `AguiEventSink` replay from the existing unified event bus.
- Adapter responses must use `ApiResponse`.
- Converted events must include event id, type, event name, run code, stream key, occurred timestamp, outcome, and metadata.
- Converted events must omit prompt text, completion text, raw tool output, API keys, tokens, and request bodies.

## Feature Model

### Actors

- AG-UI consumer: activates stream and reads event replay.
- Runtime kernel: publishes unified agent events.
- Adapter: maps bus events into AG-UI-shaped JSON.

### Stream Flow

1. Consumer activates a stream key.
2. Invocation runs with matching `conversationCode` or stream key.
3. Runtime publishes unified agent events.
4. `AguiEventSink` captures replay for the active stream key.
5. Consumer reads converted replay events.
6. Consumer deactivates the stream key.

## Page And Module Surface

- HTTP route: `/api/v1/agents/agui/streams/{streamKey}`
- HTTP route: `/api/v1/agents/agui/streams/{streamKey}/events`
- Application adapter: `tron-backend/src/application/agent.rs`
- Event source: `tron-backend/src/shared/event_bus/sink.rs`

## Repository Findings

- `AguiEventSink` already supports `activate_stream`, `deactivate_stream`, `replay`, and `snapshot`.
- The sink captures only events with stream keys and only after stream activation.
- Runtime events use `BusEventDraft::agui` and `BusRequestRef::with_stream_key`.

## Open Decisions

- None.
