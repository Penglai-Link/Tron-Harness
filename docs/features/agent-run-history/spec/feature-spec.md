# Feature Spec: Agent Run History

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: agent-run-history
- feature_name: Agent Run History
- status: approved_with_defaults
- owner: clouds3n
- source_request: complete feature 3 from `docs/brainstorm/agent-framework/prd.typ`
- main_agent: Codex lead agent on `feature/clouds3n/agent-framework/base`
- dependency_features: `react-agent-runtime`, `agent-invocation-api`, `common-event-bus`, `workspace-management`, `auth`
- dependency_contracts: `AgentInvocationService`, `AgentRunResult`, runtime identity fields, uniform `ApiResponse`, SQLite repository pattern

## Requirement

Persist safe agent run summaries and diagnostics after invocation so authenticated callers can list and inspect run outcomes without exposing prompts, raw completions, raw tool output, API keys, or sensitive event payloads.

## Product Source Slices

- prd_feature_headings: `Feature Inventory` row `Agent Run History`
- prd_rule_headings: transcript retention, masking, and replay access from the feature inventory
- prd_exception_headings: oversized transcript and missing run from the feature inventory
- dependency_order: follows Feature 1 and Feature 2

## Scope Boundary

This feature adds backend persistence and query APIs for safe run history. It does not add frontend history pages, AG-UI protocol conversion, live streaming, concrete tools, or sensitive-value materialization.

## Assumptions

- Feature1 runtime returns final output and observation counts but not a full safe event transcript.
- Feature2 invocation is the write boundary for run history.
- The first persisted history record stores summaries and diagnostics only; full transcript persistence is deferred until masking policy is richer.
- Existing auth middleware protects history reads.

## In Scope

- Agent run history domain records and status values.
- SQLite schema and repository for `agent_runs`.
- Application service methods to record completed and failed invocations.
- Protected list and detail APIs under `/api/v1/agents/runs`.
- Masking and truncation of prompt/output/error diagnostics.
- Backend tests proving persistence, paging, detail lookup, missing-run errors, and response safety.

## Out Of Scope

- Persisting full prompt text.
- Persisting full completion text.
- Persisting raw tool outputs.
- Frontend run-history pages.
- Event replay storage; feature4 owns AG-UI replay conversion.

## Constraints

- Public responses must use `{ code, message, data }`.
- Paged responses must use `{ current, size, total, records }`.
- History records must be created only after a run identity exists.
- Public JSON must not contain `apiKey`, bearer tokens, raw prompt text, raw completion text, or raw tool output.
- Detail API must return `AGENT_RUN_NOT_FOUND` for missing runs.

## Feature Model

### Actors

- Authenticated operator: lists and inspects run history.
- Invocation service: writes safe run history.
- History repository: persists and reads run summaries.

### Stored Summary

- `runCode`
- `conversationCode`
- `workspaceCode`
- `userCode`
- `appCode`
- `taskCode`
- `modelCode`
- `status`
- `outcome`
- `stepCount`
- `observationCount`
- `outputPreview`
- `errorCode`
- `errorMessage`
- `startedAt`
- `finishedAt`
- `createdAt`

## Page And Module Surface

- Backend domain/application: `tron-backend/src/application/agent.rs`, `tron-backend/src/domain/agent.rs`
- SQLite adapter: `tron-backend/src/adapters/outbound/persistence/sqlite/agent_repository.rs`
- HTTP route: `tron-backend/src/adapters/inbound/http/agents/*`
- App wiring: `tron-backend/src/app.rs`

## Repository Findings

- `ApiResponse` is already the response envelope.
- Existing services keep repositories behind `Arc<Mutex<Box<dyn ...>>>`.
- Existing SQLite repositories apply feature-local migrations on connect.
- Feature2 currently writes no history; this feature must extend invocation.

## Open Decisions

- None.
