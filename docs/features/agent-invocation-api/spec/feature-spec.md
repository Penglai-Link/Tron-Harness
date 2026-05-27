# Feature Spec: Agent Invocation API

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: agent-invocation-api
- feature_name: Agent Invocation API
- status: approved_with_defaults
- owner: clouds3n
- source_request: complete feature 2 from `docs/brainstorm/agent-framework/prd.typ`
- main_agent: Codex lead agent on `feature/clouds3n/agent-framework/base`
- dependency_features: `react-agent-runtime`, `common-event-bus`, `model-management`, `variable-management`, `skill-registry`, `workspace-management`, `app-management`, `auth`
- dependency_contracts: `ReactAgent`, `AgentRunRequest`, `AgentRunContext`, `AgentModelConfig`, `ToolRegistry`, `EventBusHandle`, Axum protected-route auth context, uniform `ApiResponse`

## Requirement

Expose a protected backend invocation surface that lets authenticated callers start an in-process ReAct run through the runtime kernel. The API resolves and freezes only the platform data current repositories can support, returns a uniform response body, and keeps HTTP concerns separate from runtime execution.

## Product Source Slices

- brainstorming_source: `docs/brainstorm/agent-framework/2026-05-26-22-18-36.md`
- prd_feature_headings: `Feature Inventory` row `Agent Invocation API`
- prd_rule_headings: `Rule: Backend Agent Framework - Run Isolation`, `Rule: Backend Agent Framework - Context Snapshot`, `Rule: Backend Agent Framework - Frozen Model Config`, `Rule: Backend Agent Framework - Event Payload Boundary`
- prd_exception_headings: `Exception: Backend Agent Framework - Invalid Run Configuration`, `Exception: Backend Agent Framework - Model Failure`, `Exception: Backend Agent Framework - Max Step Exhaustion`, `Exception: Backend Agent Framework - Event Publish Failure`

## Scope Boundary

This feature adds protected backend API and service orchestration around the runtime kernel. It does not add frontend UI, database persistence, AG-UI protocol conversion, concrete production tools, Docker/external runtime, or sensitive-variable handling.

## Assumptions

- Feature1 runtime contracts are present and tested before this API is accepted.
- Existing auth middleware provides authenticated user context on protected routes.
- Existing service layer can provide workspace validation, model defaults/catalog data, effective variables, workspace skills, and app metadata.
- If no production tools are configured, invocation can run model-only flows or test-registered tools.
- The first API can support an optional direct model config override for deterministic tests and development use while repository-backed default resolution matures.

## In Scope

- Protected route under `/api/v1/agents/runs`.
- Request DTO for prompt, workspace code, optional conversation code, optional app/task/execution scope, optional direct model config, optional limits, optional tool definitions, and dry-run/test mode flags where useful.
- Service orchestration that creates run code, validates workspace/app inputs, resolves effective variables and skills where possible, builds frozen runtime context, and invokes `ReactAgent`.
- Uniform response envelope with run code, conversation code, final output, event count or step count, and status.
- HTTP error mapping for invalid request, missing model config, runtime failure, and upstream model failure.
- Backend API tests with scripted model client support so no external provider is required.

## Out Of Scope

- Frontend page for launching agents.
- Persistent run history or transcript retrieval.
- AG-UI streaming endpoint.
- Concrete code execution tools.
- Sensitive variable resolution or masking beyond runtime event guardrails.
- Long-running async job orchestration.

## Constraints

- API responses must use the project-wide `ApiResponse` shape.
- Protected route must require auth like existing workspace/model/task routes.
- The service must freeze context before runtime starts and must not let later repository changes mutate the active run.
- Public JSON must not expose raw API keys.
- Browser verification can only claim general app smoke because this feature has no user-facing page.

## Feature Model

### Actors

- Authenticated operator or platform maintainer: starts an agent run.
- Invocation service: validates inputs, resolves available platform state, freezes context, and calls the runtime.
- Runtime kernel: executes the run and returns the result.
- Event bus: records runtime lifecycle metadata.

### Request Shape

- `prompt`: required user prompt.
- `workspaceCode`: required workspace identity.
- `conversationCode`: optional frontend thread identity.
- `appCode`: optional app identity.
- `taskCode`: optional task identity.
- `repositoryUrl`, `localDirectory`, `branchName`: optional execution scope fields derived from app data or request overrides.
- `model`: optional direct model config for first-version invocation.
- `limits`: optional max step and token limits.
- `tools`: optional allowed tool definitions for request-time test or development use.

### Response Shape

- `runCode`
- `conversationCode`
- `workspaceCode`
- `status`
- `finalOutput`
- `stepCount`

## Page And Module Surface

- Backend routes: `tron-backend/src/adapters/inbound/http/agents/*`
- App route wiring: `tron-backend/src/adapters/inbound/http.rs` and `tron-backend/src/app.rs`
- Application service: `tron-backend/src/application/agent.rs`
- Runtime dependency: `tron-backend/src/domain/agent.rs`
- No frontend page.

## Repository Findings

- Protected routes are merged under `route_layer(middleware::from_fn_with_state(... require_auth))`.
- `ApiResponse` already enforces `{ code, message, data }`.
- `AppState` already exposes model, variable, skill, workspace, app, and event-bus services.
- Existing model views mask API keys, so direct public response DTOs must not echo raw model API keys.
- App records provide `repository_url`, `local_directory`, and `default_branch`, but not repository code or worktree path.
- Browser auth is not fully mock-backed in frontend-only mode, so browser validation must use a running backend or explicit login path.

## Open Decisions

- None. Build a protected backend API and keep user-facing UI deferred.
