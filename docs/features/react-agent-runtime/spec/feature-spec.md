# Feature Spec: React Agent Runtime Kernel

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: react-agent-runtime
- feature_name: React Agent Runtime Kernel
- status: approved_with_defaults
- owner: clouds3n
- source_request: complete feature 1 from `docs/brainstorm/agent-framework/prd.typ`
- main_agent: Codex lead agent on `feature/clouds3n/agent-framework/base`
- dependency_features: `common-event-bus`, `model-management`, `variable-management`, `skill-registry`, `workspace-management`, `app-management`, `auth`
- dependency_contracts: `EventBusHandle`, `BusEventDraft`, `RecordingEventSink`, model config records, effective variable records, workspace skill bindings, app/workspace/user code identity, Makefile backend validation

## Requirement

Deliver the backend-only foundational runtime kernel for an in-process ReAct agent. The kernel owns frozen per-run context, runtime identity, production model-client contracts, tool registry contracts, bounded ReAct execution, metadata-only event publishing, typed errors, and deterministic backend tests.

## Product Source Slices

- brainstorming_source: `docs/brainstorm/agent-framework/2026-05-26-22-18-36.md`
- prd_feature_headings: `Feature: Backend Agent Framework - Run Context Model`, `Feature: Backend Agent Framework - Model Client`, `Feature: Backend Agent Framework - Tool Registry`, `Feature: Backend Agent Framework - ReAct Runtime Events`
- prd_rule_headings: `Rule: Backend Agent Framework - Run Isolation`, `Rule: Backend Agent Framework - Context Snapshot`, `Rule: Backend Agent Framework - Frozen Model Config`, `Rule: Backend Agent Framework - Tool Allow List`, `Rule: Backend Agent Framework - Event Payload Boundary`, `Rule: Backend Agent Framework - Skill Tool Separation`, `Rule: Backend Agent Framework - Step Limit`, `Rule: Backend Agent Framework - Test Substitute Boundary`
- prd_exception_headings: `Exception: Backend Agent Framework - Invalid Run Configuration`, `Exception: Backend Agent Framework - Model Failure`, `Exception: Backend Agent Framework - Missing Tool`, `Exception: Backend Agent Framework - Unauthorized Tool`, `Exception: Backend Agent Framework - Tool Failure`, `Exception: Backend Agent Framework - Max Step Exhaustion`, `Exception: Backend Agent Framework - Event Publish Failure`

## Scope Boundary

This feature is backend-internal and foundational. It does not expose HTTP routes, frontend UI, database persistence, AG-UI protocol conversion, Docker/external Codex execution, concrete platform tools, or sensitive-variable semantics.

## Assumptions

- Existing common event bus is the only event transport for runtime lifecycle events.
- Existing model, variable, skill, workspace, app, and user services are sufficient foundations, but the runtime kernel accepts frozen snapshots instead of resolving repositories itself.
- Skills and tools stay separate: skills influence instructions, tools provide executable schemas and handlers.
- Tests may inject scripted model clients and tools; production runtime still depends on real trait boundaries and a real OpenAI-compatible client implementation.
- Agent events are metadata-only and must avoid full prompt, completion, raw tool output, tokens, secrets, request bodies, and response bodies.

## In Scope

- `tron-backend/src/domain/agent.rs` or equivalent exported backend module for runtime types and errors.
- Agent identity types including separate conversation and run identity.
- Frozen `AgentRunContext` with model config, variables, skills, tools, execution scope, limits, messages, observations, and start time.
- `ModelClient` trait plus OpenAI-compatible request builders for Chat Completions, Responses, and Embeddings.
- `Tool` trait, `ToolRegistry`, tool allow-list validation, tool request, and tool observation contracts.
- `ReactAgentBuilder` and `ReactAgent` bounded loop.
- Agent event publishing through `EventBusHandle`.
- Deterministic unit tests for context isolation, model requests, tool registry behavior, event order, max-step exhaustion, and concurrency.

## Out Of Scope

- HTTP invocation API; that belongs to `agent-invocation-api`.
- Run or transcript persistence.
- Frontend pages or browser-specific behavior.
- AG-UI protocol adapter.
- Docker, shell, external Codex, or worktree tool execution.
- Sensitive variable references, secret injection, and redaction beyond event payload guardrails.
- Persisted skill-to-tool bindings.

## Constraints

- Always validate through Makefile targets before direct backend commands when a Makefile target exists.
- Runtime context must not store database handles, repository handles, event bus handles, or real tool executors.
- Production model code must not be fake-only.
- Event fields must fit existing `BusEventDraft` limits.
- The bounded loop must stop when limits are exhausted.
- Shared runtime dependencies may be cloned, but per-run messages, observations, variables, skills, tools, and step indexes must not be shared.

## Feature Model

### Actors

- Internal caller: future service that starts an agent run.
- ReAct runtime: builds model input, selects tools, records observations, and stops on output or error.
- Model client: performs provider calls using frozen model config.
- Tool registry: validates and executes allowed tools.
- Event bus: receives metadata-only runtime events.

### Core Objects

- `AgentKind`: first value `React`.
- `AgentRuntimeKind`: first value `InProcess`.
- `AgentRunRequest`: prompt, identity, workspace/user/app scope, and context inputs.
- `AgentRunContext`: immutable run snapshot plus mutable per-run message/observation vectors owned by the run.
- `AgentModelConfig`: provider protocol, base URL, API path, API key, model name, generation options, timeout, and retry fields.
- `EffectiveVariableSet`: frozen key/value/source records.
- `EffectiveSkillSet`: frozen skill names, descriptions, and instructions.
- `EffectiveToolSet`: frozen allowed tool names, descriptions, schemas, skill mapping, and required variable keys.
- `ToolRegistry`: executable handlers keyed by tool name.
- `AgentEvent`: event builder that maps runtime lifecycle metadata to the unified event bus.
- `AgentError`: typed runtime failure contract.

### State Flow

1. Build or receive `AgentRunRequest`.
2. Freeze `AgentRunContext`.
3. Publish run started event.
4. Build model input from prompt, skills, tools, variables, prior messages, and observations.
5. Request model output.
6. If final output arrives, publish output and completion events.
7. If a tool call arrives, validate allowed tool, execute through registry, append observation, and continue.
8. If a limit or failure occurs, publish failure event and return typed error.

## Page And Module Surface

- Backend module surface: `tron-backend/src/domain/agent.rs`, `tron-backend/src/domain.rs`, and backend tests.
- Event integration: `tron-backend/src/shared/event_bus/*` reused as-is unless foundation verification proves a gap.
- Model reference: `tron-backend/src/application/model.rs` verification code informs request shape only; runtime does not query model repositories.
- No frontend route or page surface.

## Repository Findings

- `EventBusHandle` is already available from `AppState` and can publish `BusEventDraft::agui` or `BusEventDraft::observe` events.
- Event validation rejects sensitive key names including prompt, completion, token, and secret; agent events must use metadata fields and summaries.
- Model records include protocol, base URL, API path, API key, model name, temperature, max tokens, top-p, timeout, and retry fields.
- Public model views mask API keys; runtime snapshots must be built from trusted backend data or supplied directly by callers.
- Effective variables already resolve workspace inheritance but have no sensitive flag.
- Skill registry can resolve enabled skills but has no executable tool schema contract.
- App records include local directory, repository URL, and default branch; no repository code or worktree path exists today.

## Open Decisions

- None. Runtime receives frozen snapshots in this feature; repository-backed resolution is handled by `agent-invocation-api` only where existing services support it.
