# Runtime V2 Architecture: Unified Agent Runtime Layer

## Status

- status: draft-ready-for-implementation
- scope: backend runtime evolution for chat and agent execution
- compatibility_target: no breaking change for existing `/api/v1/chats/*` and `/api/v1/agents/*` in phase M1

## Problem Statement

Current backend runtime behavior is functional but implementation coupling is high:

1. Invocation orchestration and runtime adapter details are mixed in the same service path.
2. Runtime capability contracts (`resume`, `cancel`, stateful session mode) are implicit instead of explicit.
3. Chat session mapping currently persists one system session id, but lacks a generalized runtime session mapping model.
4. AGUI conversion is available, but there is no formal canonical event contract between runtime internals and AGUI translator.

## Design Goals

1. Introduce a stable runtime abstraction usable by chat and non-chat invocation surfaces.
2. Separate orchestration policy from runtime adapter implementation details.
3. Support both ephemeral one-shot execution and stateful multi-turn continuation.
4. Keep AGUI as the single web-facing event protocol while preserving an internal canonical event model.
5. Allow incremental rollout without blocking current `main-agent`, `codex`, and `opencode` behavior.

## Non-Goals

1. No immediate migration to app-server adapters in M1.
2. No mandatory websocket switch; SSE remains valid.
3. No immediate cross-runtime distributed job scheduler.

## Target Topology

```text
Web UI
  |
  | SSE / WS (AGUI)
  v
Backend API Layer
  |
  +-- Conversation Service
  |     - sessions/messages/runs/usage/approvals
  |     - runtime session mapping
  |
  +-- Agent Orchestrator
  |     - runtime routing
  |     - model policy
  |     - timeout/cancel/retry policy
  |     - lifecycle state machine
  |
  +-- Runtime Registry
  |     - adapter discovery
  |     - capability lookup
  |
  +-- AgentRuntime adapters
  |     +-- MainReactRuntime (in-process model client)
  |     +-- CodexExecRuntime
  |     +-- OpenCodeRunRuntime
  |     +-- CodexAppServerRuntime (phase M3)
  |     +-- OpenCodeServeRuntime (phase M3)
  |     +-- OpenCodeAcpRuntime (phase M4)
  |
  +-- Event Pipeline
        - canonical runtime events
        - AGUI translator
        - replay/export
```

## Core Contracts

### 1) Runtime Capability Contract

```rust
pub struct RuntimeCapabilities {
    pub supports_resume: bool,
    pub supports_cancel: bool,
    pub supports_streaming: bool,
    pub supports_ephemeral: bool,
    pub supports_workdir: bool,
}
```

### 2) Runtime Request Contract

```rust
pub struct AgentRequest {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub agent_code: String,
    pub model: AgentModelConfig,
    pub input: AgentInput,
    pub execution_scope: AgentExecutionScope,
    pub limits: AgentRunLimits,
    pub runtime_session_id: Option<String>,
}

pub struct AgentInput {
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<EffectiveTool>,
    pub variables: Vec<EffectiveVariable>,
    pub skills: Vec<EffectiveSkill>,
}
```

### 3) Runtime Trait

```rust
pub trait AgentRuntime: Send + Sync {
    fn key(&self) -> RuntimeAdapterKey;
    fn capabilities(&self) -> RuntimeCapabilities;

    fn start<'a>(
        &'a self,
        req: AgentRequest,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentRuntimeError>> + Send + 'a>>;

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        req: AgentRequest,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentRuntimeError>> + Send + 'a>>;

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentRuntimeError>> + Send + 'a>>;
}
```

Notes:

- `resume` must be explicit and capability-gated.
- `cancel` may return `Unsupported` if runtime is non-cancellable.
- `start` and `resume` both produce the same `AgentRunResult` contract.

### 4) Runtime Result Contract

```rust
pub struct AgentRunResult {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub final_output: String,
    pub step_count: u32,
    pub token_usage: Option<ModelTokenUsage>,
    pub runtime_session_id: Option<String>,
    pub observations: Vec<ToolObservation>,
}
```

## Turn State Strategy

The runtime loop stays single and protocol-agnostic. Only the continuation state changes per model protocol.

### ChatTranscriptState

- Keeps the full message transcript as the source of truth.
- Builds Chat Completions requests from the transcript and current tool observations.
- Appends tool observations back into the transcript so the next turn sees the same shared history.

### ResponsesItemState

- Keeps the current transcript slice plus a `previous_response_id` cursor.
- Builds Responses requests from the current delta items instead of replaying the full transcript on every turn.
- Carries tool observations forward as function-call output items and advances the cursor after each model response.

### Shared Loop Contract

- Step counting stays in the shared loop.
- Tool batching and parallel execution stay in the shared loop.
- Failure handling and event publication stay in the shared loop.
- Only request construction, response continuation, and tool-observation propagation are protocol-specific.

## Adapter Decomposition

## MainReactRuntime

- Runtime kind: in-process
- Model path: current `HttpModelClient`
- Supports:
  - `resume`: logical continuation by passing conversation context
  - `cancel`: optional, default unsupported in M1

## CodexExecRuntime

- Runtime kind: external CLI exec
- Command model:
  - first turn: `codex exec ...`
  - resume turn: `codex exec resume ... <session_id> <prompt>`
- `runtime_session_id` source: parsed from CLI structured output

## OpenCodeRunRuntime

- Runtime kind: external CLI run
- Command model:
  - first turn: `opencode run ...`
  - resume turn: `opencode run --session <session_id> ...`
- `runtime_session_id` source: parsed from CLI structured output

## CodexAppServer/OpenCodeServe/OpenCodeAcp (future)

- Runtime kind: stateful service mode
- Session model: native thread/turn ids
- Stronger `cancel` guarantees and run multiplexing

## Orchestrator Policy

Routing is performed by `AgentOrchestrator` rather than adapters:

1. Resolve workspace + agent enablement.
2. Resolve model policy by agent type:
   - `main-agent` -> platform/workspace model catalog
   - `system-agent` -> runtime scanned model catalog
3. Select adapter key:
   - `main-agent` => `MainReactRuntime`
   - `codex` => `CodexExecRuntime`
   - `opencode` => `OpenCodeRunRuntime`
4. Decide `start` vs `resume`:
   - if `runtime_session_id` exists and adapter supports resume -> `resume`
   - else -> `start`
5. Apply timeout/cancel/retry policy envelope.

## Canonical Event Model

Runtime internal events become a strict canonical contract before AGUI translation.

```text
RUN_REQUESTED
RUN_STARTED
MODEL_REQUESTED
MODEL_RESPONDED
TOOL_SELECTED
TOOL_STARTED
TOOL_COMPLETED
TOOL_FAILED
OUTPUT_DELTA
OUTPUT_COMPLETED
RUN_COMPLETED
RUN_FAILED
RUN_CANCELLED
```

Rules:

1. Canonical events carry normalized metadata only; no prompt/raw secret payload.
2. AGUI translator is the only path that converts canonical events into AGUI protocol events.
3. Replay/export use canonical storage as source of truth.

## Conversation Mapping Model

Current single-field mapping evolves to generalized mapping.

### New Table (proposed)

```text
conversation_runtime_sessions
- id
- conversation_code
- runtime_adapter_key
- agent_code
- runtime_session_id
- status            (ACTIVE|CLOSED|FAILED)
- created_at
- updated_at
- last_run_code
```

### Constraints

1. Unique active row per `(conversation_code, runtime_adapter_key, agent_code)`.
2. `chat_sessions.system_agent_session_id` remains as compatibility field in M1/M2.
3. Read priority during migration:
   - new mapping table first
   - fallback to legacy session field

## API Evolution

M1 keeps existing APIs compatible.

### Existing endpoints (compatible)

- `POST /api/v1/chats/sessions/{session_code}/messages`
- `POST /api/v1/chats/sessions/{session_code}/messages/stream`
- `POST /api/v1/agents/runs`

### New optional fields (phase-gated)

```json
{
  "runtime": {
    "adapterKey": "CODEX_EXEC",
    "sessionId": "thr_xxx",
    "mode": "STATEFUL"
  }
}
```

### New runtime introspection endpoints (M2)

- `GET /api/v1/agents/runtimes`
- `GET /api/v1/agents/runtimes/{adapter_key}/capabilities`

### New runtime control endpoints (M2/M3)

- `POST /api/v1/agents/runs/{run_code}/cancel`
- `POST /api/v1/agents/runs/{run_code}/resume`

## Lifecycle State Machine

```text
CREATED -> RUNNING -> COMPLETED
                |-> FAILED
                |-> CANCELLING -> CANCELLED
```

Rules:

1. Only RUNNING can transition to CANCELLING.
2. Resume always creates a new run record linked to existing runtime session.
3. `runtime_session_id` may rotate; latest active mapping is authoritative.

## Phased Delivery Plan

## M1 (now)

1. Add `AgentRuntime` abstraction and runtime registry.
2. Wrap current main/system behavior behind runtime adapters.
3. Preserve all existing API response shapes.
4. Keep existing AGUI output stable.

## M2

1. Introduce canonical event persistence contract.
2. Add runtime capabilities introspection endpoints.
3. Add cancel/resume public APIs (capability-gated).

## M3

1. Introduce app-server/serve adapters.
2. Add generalized conversation runtime session mapping table.
3. Shift stateful chat sessions to service-mode adapters where available.

## M4

1. Add ACP adapter support.
2. Add adapter policy plugin points.
3. Deprecate legacy direct session field and compatibility fallback.

## Backward Compatibility

1. Existing chats continue to work with legacy `system_agent_session_id`.
2. Existing frontend can ignore new `runtime` block fields.
3. New abstractions should be internal-first; external contract changes are additive.

## Risks and Mitigations

1. Risk: adapter behavior divergence.
- Mitigation: adapter contract tests with shared fixtures per canonical event sequence.

2. Risk: resume mismatch across adapters.
- Mitigation: capability flags + strict error mapping (`ResumeUnsupported`, `SessionExpired`).

3. Risk: AGUI regressions.
- Mitigation: golden AGUI protocol event snapshot tests driven by canonical events.

## Test Strategy Updates

1. Contract tests per adapter: start/resume/cancel/capabilities.
2. Orchestrator tests: routing/model policy/timeout envelope.
3. Migration tests: legacy session id fallback and mapping table precedence.
4. AGUI translation tests: canonical -> AGUI deterministic mappings.

## Immediate Implementation Checklist

1. Create `application/agent/turn_state.rs` with chat transcript and Responses item continuation strategies.
2. Create `application/agent/runtime_adapter.rs` with trait and capability structs.
3. Create `application/agent/runtime_registry.rs` with adapter selection.
4. Move invocation runtime selection from `selected_model_client` to runtime registry.
5. Keep `SystemAgentCliModelClient` as adapter dependency, not orchestration entrypoint.
6. Add minimal runtime adapter key and capability fields to run results.
