= PRD: Agent Framework

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron backend internal agent framework.
- Problem: the platform has model, variable, skill, app, workspace, and event capabilities, but no isolated backend runtime contract for executing the platform main agent.
- Target users: platform maintainers, backend developers, future AG-UI consumers, and workspace operators whose actions will later invoke agents.
- Outcome: the backend gains a production-capable in-process ReAct agent kernel with frozen per-run context, OpenAI-compatible model access, tool execution, and unified runtime events.

== Background
- Why now: workspace, app, model, variable, skill, and event foundations are already present, so agent execution can now reuse stable platform concepts instead of inventing a separate runtime.
- Current pain: agent execution has no canonical run context, no production model client boundary, no allowed-tool snapshot, and no event lifecycle for future UI streaming or observability.
- Business context: this is a platform foundation for later main-agent invocation, AG-UI event adaptation, run history, and external runtime execution.

== Goals
- Define one backend framework module for agent identity, run context, model config, effective variables, effective skills, effective tools, tool execution, limits, messages, observations, and errors.
- Implement the first agent type as an in-process `ReactAgent` with a bounded loop.
- Use a production `ModelClient` abstraction backed by OpenAI-compatible Chat Completions, Responses, and Embeddings APIs.
- Use a real `ToolRegistry` and `Tool` trait for executable tools.
- Publish agent lifecycle, model, tool, observation, output, and failure events through the existing unified backend event bus.
- Keep run state isolated per workspace, app, user, conversation, and run.
- Cover builder validation, model request construction, tool calls, event order, limit exhaustion, and concurrent runs with backend tests.

== Scope
- In scope: backend-only framework code, unit-test-only model substitute, OpenAI-compatible model client request construction, real tool registry, ReAct runtime loop, event publishing, frozen run context, and backend test coverage.
- Out of scope: HTTP routes, frontend UI, Docker runtime, external Codex runtime, AG-UI protocol adapter, sensitive variable handling, run history persistence, transcript persistence, and runtime authorization policy beyond allowed tool snapshots.

== Feature Requirements
=== Module: Backend Agent Framework
==== Feature: Backend Agent Framework - Run Context Model
- name: Frozen agent run context.
- description: represent one independent run with conversation identity, run identity, workspace identity, user identity, model config, effective variables, effective skills, effective tools, optional code execution scope, runtime descriptor, limits, messages, observations, and start time.
- user or role: backend runtime and future invocation services.
- entry point: Rust domain/application module constructors and `ReactAgent::run`.
- trigger: a caller starts one agent run.
- preconditions: caller has resolved workspace, model config, variables, skills, tools, and optional execution scope.
- main flow: build `AgentRunRequest`, freeze `AgentRunContext`, pass context through model, tool, and event boundaries without querying mutable platform config during the run.
- inputs and outputs: request input becomes a context snapshot; output is the final agent result or typed `AgentError`.
- permissions or visibility: no HTTP exposure in this version.
- related page headings: none.
- related business rule headings: `Rule: Backend Agent Framework - Run Isolation`, `Rule: Backend Agent Framework - Context Snapshot`.
- related exception headings: `Exception: Backend Agent Framework - Invalid Run Configuration`.

==== Feature: Backend Agent Framework - Model Client
- name: Production model client boundary.
- description: call OpenAI-compatible Chat Completions, Responses, and Embeddings endpoints using `AgentModelConfig` from the frozen context.
- user or role: `ReactAgent`, future retrieval and memory tools.
- entry point: `ModelClient` trait and production OpenAI-compatible implementation.
- trigger: the ReAct loop requests model output, or framework consumers need embeddings.
- preconditions: model config has endpoint, provider/API mode, model name, and credentials resolved before the run starts.
- main flow: construct provider-compatible requests from context, messages, tools, and limits; return model output, tool calls, embeddings, or typed failures.
- inputs and outputs: chat/response/embedding requests in; normalized model outputs and errors out.
- permissions or visibility: test substitutes are allowed only in backend unit tests.
- related page headings: none.
- related business rule headings: `Rule: Backend Agent Framework - Frozen Model Config`, `Rule: Backend Agent Framework - Test Substitute Boundary`.
- related exception headings: `Exception: Backend Agent Framework - Model Failure`.

==== Feature: Backend Agent Framework - Tool Registry
- name: Real tool execution registry.
- description: expose allowed tool metadata to the model and execute selected tools through registered implementations.
- user or role: `ReactAgent` and platform tool providers.
- entry point: `ToolRegistry`, `Tool`, `ToolRequest`, and `ToolObservation`.
- trigger: model output selects a tool call.
- preconditions: selected tool appears in the frozen `EffectiveToolSet` and the registry has an executor for that tool name.
- main flow: validate the tool is allowed, publish selected/started events, execute the tool with run context, record observation, publish completion or failure.
- inputs and outputs: tool request and context in; tool observation or typed tool error out.
- permissions or visibility: authorization is bounded to the frozen allowed-tool snapshot in this version.
- related page headings: none.
- related business rule headings: `Rule: Backend Agent Framework - Tool Allow List`, `Rule: Backend Agent Framework - Skill Tool Separation`.
- related exception headings: `Exception: Backend Agent Framework - Missing Tool`, `Exception: Backend Agent Framework - Unauthorized Tool`, `Exception: Backend Agent Framework - Tool Failure`.

==== Feature: Backend Agent Framework - ReAct Runtime Events
- name: ReAct loop and unified events.
- description: execute a bounded ReAct loop and publish agent runtime events through the existing backend event bus.
- user or role: observability consumers, audit consumers, future AG-UI adapter.
- entry point: `ReactAgentBuilder` and `ReactAgent::run`.
- trigger: caller invokes the agent with a prompt and run context input.
- preconditions: builder has a model client, tool registry, event publisher, and valid defaults.
- main flow: publish run start, build model input, request model output, execute allowed tools when requested, create observations, stop on final output or limit, and publish completed or failed events.
- inputs and outputs: run request in; final output or failure out; runtime events published.
- permissions or visibility: events are internal backend events and must not include full prompt text, full completion text, or raw tool output.
- related page headings: none.
- related business rule headings: `Rule: Backend Agent Framework - Event Payload Boundary`, `Rule: Backend Agent Framework - Step Limit`.
- related exception headings: `Exception: Backend Agent Framework - Max Step Exhaustion`, `Exception: Backend Agent Framework - Event Publish Failure`.

== Page Prototypes And Interactions
=== Page: Backend API - Internal Rust Framework
- entry: no HTTP route or frontend page in this version.
- layout: Rust modules under the backend domain/application boundary, aligned with existing repository conventions.
- data display: not applicable.
- actions: build run context, call model, execute tools, publish events, return final output.
- states: started, model requested, model responded, tool selected, tool started, tool completed, tool failed, observation created, output created, completed, failed.

==== Interaction: Backend API - Run React Agent
- trigger control: internal caller invokes `ReactAgent::run`.
- preconditions: valid request, frozen context inputs, model client, tool registry, event publisher, and run limits.
- button or control behavior: not applicable.
- form rules: not applicable.
- confirmation behavior: not applicable.
- success feedback: returned final output and `agent.run.completed` event.
- failure feedback: typed `AgentError` and `agent.run.failed` event.
- navigation or refresh behavior: not applicable.

== Business Rules
=== Rule: Backend Agent Framework - Run Isolation
- condition: multiple workspaces, users, apps, or conversations run agents concurrently.
- decision or calculation: each call creates an independent `AgentRunContext`.
- state transition: shared dependencies may be reused, but messages, observations, step index, variables, skills, tools, and run limits are per-run state.
- permission or data boundary: one run must not read or mutate another run's mutable runtime state.
- conflict or priority rule: shared tool implementations own their own synchronization when they hold shared resources.
- example: two concurrent runs through one `ReactAgent` must keep distinct observations and event identities.

=== Rule: Backend Agent Framework - Context Snapshot
- condition: a run starts after model, variable, skill, and tool resolution.
- decision or calculation: the runtime uses frozen values from `AgentRunContext` for the full run.
- state transition: later platform configuration changes do not affect the active run.
- permission or data boundary: runtime dependencies such as database handles and event bus handles do not live inside `AgentRunContext`.
- conflict or priority rule: run context values win over any mutable external config lookup during execution.
- example: model code and variable values used at step 3 match the values captured before step 1.

=== Rule: Backend Agent Framework - Frozen Model Config
- condition: `ModelClient` needs to call a model.
- decision or calculation: it uses `AgentModelConfig` from the current context and does not query model management repositories during the run.
- state transition: model request construction is deterministic for the frozen context plus current messages/tools.
- permission or data boundary: credentials or endpoint data are supplied by the resolved config.
- conflict or priority rule: test model clients may exist only in tests.
- example: Chat Completions and Responses requests both read model name and endpoint from the context.

=== Rule: Backend Agent Framework - Tool Allow List
- condition: the model returns a tool call.
- decision or calculation: the runtime checks the tool name against `AgentRunContext.tools` before registry execution.
- state transition: allowed tools may run; disallowed tools fail the run or step with a typed error.
- permission or data boundary: registry availability alone does not grant tool access.
- conflict or priority rule: the frozen effective tool set is the authority for a run.
- example: a registry contains `shell.exec`, but the run can execute it only when the context allowed-tool snapshot includes `shell.exec`.

=== Rule: Backend Agent Framework - Event Payload Boundary
- condition: the runtime publishes agent events.
- decision or calculation: events include identity, step, model, tool, skill, app, repository, branch, task, runtime kind, and outcome fields.
- state transition: lifecycle events are emitted in order for started, model, tool, observation, output, completed, and failed states.
- permission or data boundary: events do not include full prompt text, full completion text, or raw tool output.
- conflict or priority rule: future AG-UI adapters consume the unified event bus rather than a separate agent event bus.
- example: `agent.tool.completed` includes `tool_name`, `run_code`, and `outcome`, but not the full tool response body.

=== Rule: Backend Agent Framework - Skill Tool Separation
- condition: effective skills and effective tools are loaded for a run.
- decision or calculation: skills contribute instructions and tool names; tools define callable schemas and executable handlers.
- state transition: model input may include skill instructions and tool schemas, while execution goes through `ToolRegistry`.
- permission or data boundary: a skill name alone does not execute code.
- conflict or priority rule: an executable tool must exist in both the allowed tool snapshot and registry.
- example: a skill can mention `workspace.search`, but the runtime executes only the `workspace.search` tool definition.

=== Rule: Backend Agent Framework - Step Limit
- condition: the ReAct loop reaches a configured maximum step count.
- decision or calculation: stop execution and return max-step exhaustion.
- state transition: publish `agent.run.failed`.
- permission or data boundary: no further model or tool calls happen after exhaustion.
- conflict or priority rule: limits protect backend process health over model continuation.
- example: a looping tool-call sequence stops at the configured limit.

=== Rule: Backend Agent Framework - Test Substitute Boundary
- condition: backend tests need deterministic model outputs.
- decision or calculation: tests may inject a substitute `ModelClient`.
- state transition: production code continues to depend on the same trait boundary and real implementation.
- permission or data boundary: fake-only production paths are not allowed.
- conflict or priority rule: substitutes are scoped to unit tests.
- example: a ReAct tool-call test returns a scripted tool call then final answer.

== Exceptions And Boundaries
=== Exception: Backend Agent Framework - Invalid Run Configuration
- scenario: required run context inputs, model config, or limits are missing or invalid.
- expected product behavior: builder or run preparation returns a typed invalid-configuration error.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: no run state is persisted.
- retry, recovery, or compensation behavior: caller must resolve missing configuration and retry.
- logging/audit expectation: failure can be represented by `agent.run.failed` only after a run identity exists.

=== Exception: Backend Agent Framework - Model Failure
- scenario: the model endpoint rejects the request, times out, or returns an unsupported payload.
- expected product behavior: runtime returns a typed model failure and publishes failure lifecycle events.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: observations already created remain in memory for the run result path only.
- retry, recovery, or compensation behavior: no automatic retry in the first version unless existing HTTP client conventions already provide it.
- logging/audit expectation: event payload records model identity and outcome without raw prompt or completion text.

=== Exception: Backend Agent Framework - Missing Tool
- scenario: a selected tool appears in the allowed-tool snapshot but no registry executor exists.
- expected product behavior: runtime returns a typed missing-tool error and publishes `agent.tool.failed` followed by `agent.run.failed`.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: no synthetic successful observation is created.
- retry, recovery, or compensation behavior: register the missing tool and retry the run.
- logging/audit expectation: event payload includes `tool_name` and outcome.

=== Exception: Backend Agent Framework - Unauthorized Tool
- scenario: model selects a tool not included in the frozen effective tool set.
- expected product behavior: runtime rejects the call before registry execution and publishes failure events.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: unauthorized tool side effects must not happen.
- retry, recovery, or compensation behavior: caller must resolve skill/tool binding before retrying.
- logging/audit expectation: record tool name and denied outcome without raw model content.

=== Exception: Backend Agent Framework - Tool Failure
- scenario: a registered and allowed tool returns an execution error.
- expected product behavior: runtime publishes `agent.tool.failed` and handles the error according to the ReAct runtime policy for this version.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: failed tool output is not treated as a successful observation.
- retry, recovery, or compensation behavior: no implicit retry unless the tool implementation owns it.
- logging/audit expectation: include tool identity, step index, and failure outcome.

=== Exception: Backend Agent Framework - Max Step Exhaustion
- scenario: model keeps requesting tools or does not return final output before the limit.
- expected product behavior: runtime stops and returns a max-step exhaustion error.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: no extra model or tool call occurs after the limit.
- retry, recovery, or compensation behavior: caller may retry with adjusted prompt or limits.
- logging/audit expectation: publish `agent.run.failed` with exhaustion outcome.

=== Exception: Backend Agent Framework - Event Publish Failure
- scenario: event publisher fails while the run is active.
- expected product behavior: runtime returns a typed event failure unless existing event bus semantics define fire-and-forget behavior.
- user-visible message or fallback: no frontend message in this version.
- data consistency expectation: the run does not claim completed if required lifecycle events cannot be published.
- retry, recovery, or compensation behavior: caller may retry after event infrastructure recovers.
- logging/audit expectation: preserve the original run identity in logs if available.

== Shared Requirements
- Unified event bus: shared because observability, audit, and future AG-UI all consume runtime events; consumers: React Agent Runtime Kernel, future AG-UI adapter, future run history; decision: already available as `common-event-bus`, verify and reuse before runtime work.
- Existing effective configuration services: shared because model, variable, skill, workspace, app, and user data already define the platform inputs an agent run will snapshot; consumers: React Agent Runtime Kernel and future invocation API; decision: already available enough for the kernel to accept frozen snapshots, but full repository-backed resolution stays outside the first runtime feature.
- Runtime identity model: shared because conversation, run, workspace, user, app, repository, branch, and task identities must be consistent across events and future APIs; consumers: React Agent Runtime Kernel, future invocation API, future run history, future AG-UI adapter; decision: deliver inside the first foundational feature, React Agent Runtime Kernel, before dependent features.
- Frozen agent context contract: shared because model config, variables, skills, tools, execution scope, messages, observations, and limits must be isolated per run; consumers: React Agent Runtime Kernel, future invocation API, future run history, future code runtime; decision: deliver inside React Agent Runtime Kernel before dependent features.
- Testable model boundary: shared because deterministic unit tests need scripted model outputs while production needs real provider calls; consumers: React Agent Runtime Kernel and future code/runtime tools; decision: deliver inside React Agent Runtime Kernel before dependent features.
- Tool execution contract: shared because later platform tools, code tools, and business-data tools need one executor boundary; consumers: React Agent Runtime Kernel, future code runtime, and future sensitive-variable runtime; decision: deliver inside React Agent Runtime Kernel before dependent features.
- Sensitive value handling: shared because variables and tools will eventually need secret references, masking, and redaction; consumers: future sensitive-variable runtime, future code runtime, and future invocation API; decision: defer because current variable management has no sensitive/non-sensitive distinction.

== Feature Inventory
1. React Agent Runtime Kernel: module Backend Agent Framework; source headings `Feature: Backend Agent Framework - Run Context Model`, `Feature: Backend Agent Framework - Model Client`, `Feature: Backend Agent Framework - Tool Registry`, `Feature: Backend Agent Framework - ReAct Runtime Events`; pages `Page: Backend API - Internal Rust Framework`; rules all `Rule: Backend Agent Framework - ...` headings in this PRD; exceptions all `Exception: Backend Agent Framework - ...` headings in this PRD; goal execute an isolated backend ReAct agent run using frozen runtime context, real OpenAI-compatible model client, real tool registry, bounded loop, and unified event publishing; in scope backend-only runtime, model/tool contracts, event lifecycle, concurrency isolation, and backend tests; out of scope HTTP routes, frontend UI, persistence, AG-UI adapter, Docker, external Codex runtime, concrete platform tools, and sensitive variable handling; delivery type foundational shared feature; depends on already available foundations: common event bus, model management, variable management, skill registry, workspace governance, app registry, and auth/user context; delivers shared requirements: runtime identity model, frozen agent context contract, testable model boundary, and tool execution contract.
2. Agent Invocation API: module Backend Agent Framework; source heading future invocation contract; pages future internal or HTTP invocation route; rules runtime identity and permission mapping; exceptions invalid caller, missing workspace, missing model, and unsupported runtime; goal let backend callers or future frontend/API surfaces start a main-agent run through the kernel; in scope request/response DTOs and service orchestration; out of scope first runtime kernel; depends on Feature 1.
3. Agent Run History: module Backend Agent Framework; source heading future persistence contract; pages future run detail and observability surfaces; rules transcript retention, masking, and replay access; exceptions oversized transcript and missing run; goal persist run summaries, transcript-safe artifacts, outcomes, and diagnostics; in scope database schema and query APIs; out of scope first runtime kernel; depends on Feature 1 and Feature 2.
4. Agent AG-UI Stream Adapter: module Backend Agent Framework; source heading future event adapter; pages future AG-UI consumer route; rules event filtering and protocol conversion; exceptions disconnected consumer and stale stream; goal consume unified agent events and convert them to AG-UI protocol events; in scope adapter and stream contract; out of scope first runtime kernel and history persistence; depends on Feature 1 and common event bus.
5. Agent Code Runtime: module Backend Agent Framework; source heading future code execution runtime; pages future app/task execution surface; rules repository/worktree isolation and command scope; exceptions missing repository, dirty worktree, and command failure; goal execute code-aware agent tools in Docker, external Codex, or managed worktrees; in scope runtime adapter and sandbox boundary; out of scope first runtime kernel; depends on Feature 1 and Feature 2.
6. Agent Sensitive Variable Runtime: module Backend Agent Framework; source heading future secret materialization; pages variable administration and runtime injection boundaries; rules value references, masking, and redaction; exceptions missing secret and unauthorized secret use; goal prevent plaintext secret exposure while allowing tools and models to receive authorized sensitive values; in scope secret reference handling and runtime redaction; out of scope first runtime kernel because current variable management does not expose sensitive values; depends on Feature 1 and variable management evolution.

== Dependency Map
#mermaid(
  "flowchart LR
    EF1[\"Existing Foundation: Common Event Bus\"] --> F1[\"Feature 1: React Agent Runtime Kernel\"]
    EF2[\"Existing Foundation: Effective Platform Config Services\"] --> F1
    F1 --> SR1[\"Delivered Shared Requirement: Runtime Identity Model\"]
    F1 --> SR2[\"Delivered Shared Requirement: Frozen Agent Context Contract\"]
    F1 --> SR3[\"Delivered Shared Requirement: Testable Model Boundary\"]
    F1 --> SR4[\"Delivered Shared Requirement: Tool Execution Contract\"]
    F1 --> F2[\"Feature 2: Agent Invocation API\"]
    F1 --> F3[\"Feature 3: Agent Run History\"]
    F2 --> F3
    F1 --> F4[\"Feature 4: Agent AG-UI Stream Adapter\"]
    F1 --> F5[\"Feature 5: Agent Code Runtime\"]
    F2 --> F5
    F1 --> F6[\"Feature 6: Agent Sensitive Variable Runtime\"]"
)

== Recommended Implementation Order
1. Existing shared foundation verification - confirm `common-event-bus`, model management, variable management, skill registry, workspace governance, app registry, and auth/user context are available enough for the kernel to consume frozen snapshots.
2. React Agent Runtime Kernel - this is the first bounded foundational shared feature; it completes the runtime identity model, frozen context contract, model boundary, tool execution contract, lifecycle events, and backend proof surface before dependent features start.
3. Agent Invocation API - this can expose or orchestrate the kernel only after the shared runtime contracts are proven.
4. Agent Run History - persistence should follow invocation semantics and masking decisions rather than constrain the first runtime loop.
5. Agent AG-UI Stream Adapter - protocol conversion should consume proven unified runtime events instead of creating a second event stream.
6. Agent Code Runtime - Docker, external Codex, and worktree execution should wait until the base tool/runtime contract is stable.
7. Agent Sensitive Variable Runtime - sensitive value references and redaction should evolve after current variable management gains explicit secret semantics.

== Acceptance
=== Acceptance: Backend Agent Framework - Run Context Kernel
- Backend tests prove separate runs keep distinct identity, messages, observations, limits, and effective configuration snapshots.
- The context model does not store database handles, event bus handles, real tool executors, or other shared runtime dependencies.
- Conversation identity and run identity are represented separately.

=== Acceptance: Backend Agent Framework - Model Client
- Backend tests prove Chat Completions, Responses, and Embeddings request construction uses the frozen `AgentModelConfig`.
- Unit tests can inject a model substitute without creating a fake-only production path.
- Model failures return typed errors.

=== Acceptance: Backend Agent Framework - Tool Registry
- Backend tests prove registry registration, lookup, missing-tool behavior, allowed-tool validation, and tool observations.
- Tool execution receives the run context.
- A registered tool cannot run unless it is also present in the frozen effective tool snapshot.

=== Acceptance: Backend Agent Framework - ReAct Runtime Events
- Backend tests prove final-answer flow, tool-call flow, max-step exhaustion, failure handling, and event ordering.
- Runtime events publish through the unified backend event bus and include conversation, run, workspace, user, model, tool, skill, app, repository, branch, task, runtime, step, and outcome fields when available.
- Runtime events exclude full prompt text, full completion text, and raw tool output.
- Concurrent runs through one agent instance do not leak messages, observations, step indexes, or variable snapshots.

== Risks And Open Questions
- Existing model management may not yet expose every provider field needed by all three OpenAI-compatible APIs; the first feature docs must choose whether to model the framework snapshot independently or map from current model entities immediately.
- Event bus semantics need confirmation: if publishing is fire-and-forget today, the runtime error policy for event publish failure should follow repository reality rather than inventing stronger guarantees.
- Tool authorization is intentionally limited to the frozen allowed-tool snapshot; deeper role/policy checks are deferred.
- Sensitive variable handling is deferred because current variable management does not expose sensitive versus non-sensitive values.
- No frontend or HTTP API should be claimed by this PRD slice.
