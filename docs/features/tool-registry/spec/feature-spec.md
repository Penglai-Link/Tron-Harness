# Feature Spec: Tool Registry

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: tool-registry
- feature_name: Tool Registry
- status: approved_with_defaults
- owner: clouds3n
- source_request: evolve the platform tool registry from business capabilities only to include a bounded coding domain for main-agent, expose those tools on the tools page, preserve workspace `/agents` internal-agent assignment and runtime exposure behavior, and inject effective coding tools into main-agent runtime through LLM tool calling.
- main_agent: Codex lead agent
- dependency_features: `agent-management`, `react-agent-runtime`, `workspace-management`, `model-management`, `shared-ui-feedback`
- dependency_contracts: uniform API response envelope, existing `/settings/*` shell, existing workspace `/agents` shell, backend application-service execution, current agent invocation flow

## Product Source Slices

- brainstorming_source: `docs/brainstorm/tool-registry/2026-06-01-23-36-35.md`
- source_headings:
  - `Requirement`
  - `Selected Approach`
  - `Core Concepts`
  - `Runtime Design`
  - `Backend Architecture`
  - `Page Surfaces`
  - `API Direction`
  - `Migration And Evolution Rules`

## Requirement

Tron needs the tool registry evolution to:

- keep exposing a platform tools page under settings where operators can inspect all platform tools
- keep existing backend module capabilities as code-defined business tools
- add a bounded coding domain for main-agent with file, search, patch, and command execution capabilities
- lets workspace `/agents` manage internal-agent tool policy without removing the existing system-agent controls
- starts with `main-agent` as the only internal agent
- defaults `main-agent` to all platform business tools plus the new coding-domain tools
- supports internal-agent runtime tool exposure modes `direct_list` and `domain_discovery`
- supports expanded-domain persistence settings with `sticky_expanded` default
- resolves the effective assigned tools during agent invocation and feeds them into LLM tool calling
- requires every tool input JSON Schema to expose top-level `description` and `examples`

## Scope Boundary

This feature covers code-defined tool catalog, internal-agent policy persistence and resolution, backend APIs, frontend tools page and workspace agent management evolution, runtime tool exposure logic, coding-domain executor registration, and focused tests.

## Assumptions

- V1 includes only business tools backed by Tron backend application services.
- This evolution intentionally reopens the earlier boundary and adds a bounded coding domain for host-backed tools.
- Coding-domain tools do not need sandbox or permission-policy modeling in this iteration.
- Tool definitions are code-defined rather than database-authored.
- System-agent CLI runtimes stay outside this new business-tool execution path in v1.
- `main-agent` remains the only internal agent in v1, but the model must scale to more.

## In Scope

- code-backed domain and concrete tool catalog
- coding domains `bash`, `fs`, and `search`
- concrete coding tools:
  - `bash.exec`
  - `fs.read_text`
  - `fs.write_text`
  - `fs.apply_patch`
  - `fs.list`
  - `fs.stat`
  - `search.grep`
  - `search.glob`
- tool catalog read APIs
- workspace internal-agent policy APIs and persistence
- `main-agent` default policy and effective assignment resolution
- runtime support for `direct_list`
- runtime seam and discovery tool contract for `domain_discovery`
- Rust-native file executor implementations for `fs.*`
- system-command executor implementations for `bash.exec`, `search.grep`, and `search.glob`
- platform `/settings/tools`
- workspace `/agents` internal-agent section plus existing system-agent section

## Out Of Scope

- `project.make`
- user-authored tool definitions
- external plugin/MCP tool federation
- system-agent CLI runtime consuming the new business tool registry in v1
- dedicated per-command tools for Unix utilities such as `sed`, `awk`, `cat`, `head`, `tail`, `jq`, or `git`
- sandbox and permission controls for coding tools

## Constraints

- All HTTP responses keep the repo-wide response envelope.
- Backend business tools must execute through application services, not HTTP self-calls.
- Coding tools must execute through typed backend executors rather than proxying through Tron HTTP endpoints.
- Existing system-agent APIs and behaviors must keep working.
- Workspace `/agents` can no longer be treated as system-only after this evolution.
- `all_platform_tools` means all currently registered available business tools.
- For this evolution, `all_platform_tools` also includes all currently registered available coding tools.
- `selected_tools` with an empty selected set means no effective tools.
- Every tool input schema must include top-level `description` and `examples`.
- Common Unix utilities such as `sed` are modeled through `bash.exec(command,args[])`, not as dedicated catalog tools.

## Feature Model

### Entities

- `ToolDomainDefinition`
- `ToolDefinition`
- `InternalAgentPolicy`
- `WorkspaceInternalAgentPolicy`
- `WorkspaceInternalAgentToolAssignment`
- `AgentToolExposureSnapshot`

### Policies

- Tool definitions are registered in backend code and grouped by domain.
- Persistence stores internal-agent policy and explicit assignments only.
- `main-agent` inherits all platform tools by default.
- Workspace policy may override assignment mode, exposure mode, and expanded-domain persistence mode.
- Runtime must distinguish assigned tools from materialized currently visible tool schemas.
- Coding-domain tool boundaries are:
  - Rust-native file primitives for `fs.*`
  - system command execution for `bash.exec`
  - system `rg`-backed discovery for `search.*`

## Page And Module Surface

- Backend modules: `tron-backend/src/application/tool_registry/**`, `tron-backend/src/application/agent/**`, `tron-backend/src/domain/agent/**`, `tron-backend/src/adapters/inbound/http/agents/**`, new tools routes as needed
- Frontend pages:
  - `/settings/tools`
  - `/agents`
- Shared backend route surfaces:
  - `GET /api/v1/tools/domains`
  - `GET /api/v1/tools`
  - `GET /api/v1/tools/{toolCode}`
  - workspace internal-agent policy endpoints under `/api/v1/agents/workspaces/{workspaceCode}/internal/*`

## Repository Findings

- Backend already has a runtime executor registry named `ToolRegistry`, so catalog-layer naming should avoid collision or rename the execution layer.
- `AgentRunContext.tools` currently holds a flat concrete schema list and doubles as the allow-list, so runtime needs a new separation between assigned and materialized tools.
- Frontend already has agent-management page shells and table/search patterns that can be reused for both tools and internal-agent controls.
- Existing backend tests assert `main-agent` is absent from workspace system-agent rows, so new internal-agent APIs should be parallel to the system-agent API family rather than mutating those rows in place.
- Existing tool catalog definitions currently localize field-level descriptions but do not guarantee top-level schema `description` plus `examples`, so coding-domain schema requirements must extend the catalog materialization contract.

## Open Decisions

- None.
