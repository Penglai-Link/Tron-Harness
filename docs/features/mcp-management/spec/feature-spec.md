# Feature Spec: MCP Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: mcp-management
- feature_name: MCP Management
- status: approved_with_defaults
- owner: clouds3n
- source_request: add an `mcp-management` module that gives Tron a platform MCP gateway, platform-owned remote MCP server registry, workspace-effective MCP server policy, internal-agent MCP server assignment, and runtime MCP tool federation aligned with current auth and HTTP client constraints
- main_agent: Codex lead agent
- dependency_features: `auth`, `workspace-management`, `agent-management`, `tool-registry`, `agent-invocation-api`, `react-agent-runtime`, `shared-ui-feedback`
- dependency_contracts: authenticated settings shell, authenticated workspace shell, active-workspace validation, uniform API response envelope, existing internal-agent policy APIs, current runtime invocation path, shared HTTP client profiles

## Product Source Slices

- brainstorming_source: `docs/brainstorm/mcp-management/2026-06-05-23-07-39.md`
- prd_source: `docs/brainstorm/mcp-management/prd.typ`
- source_headings:
  - `Feature: MCP Management - Platform MCP Gateway`
  - `Feature: MCP Management - Platform MCP Server Registry`
  - `Feature: MCP Management - Workspace Effective MCP Servers`
  - `Feature: MCP Management - Internal Agent MCP Server Access`
  - `Feature: MCP Management - Runtime MCP Tool Federation`
  - `Page: /settings/mcp-servers - Platform MCP Settings`
  - `Page: /mcp-servers - Workspace MCP Servers`
  - `Page: /agents - Workspace Agent Management`
  - `Rule: MCP Management - Platform Server Ownership`
  - `Rule: MCP Management - Workspace Enable Disable Override`
  - `Rule: MCP Management - Agent Assignment By Server`
  - `Rule: MCP Management - Runtime Tool Materialization`
  - `Rule: MCP Management - Disable Versus Delete`
  - `Exception: MCP Management - Deleted Selected Server`
  - `Exception: MCP Management - Runtime Discovery Failure`

## Requirement

Tron needs one bounded `mcp-management` feature that introduces two operator-facing management surfaces, one internal-agent policy extension, and one runtime effect.

- Platform settings at `/settings/mcp-servers` must let an operator control the inbound variables MCP gateway and manage platform-owned remote MCP server definitions with stable create, edit, disable, enable, health-check, and delete behavior.
- Workspace management at `/mcp-servers` must show only inherited platform MCP servers for the active workspace. The workspace may disable one inherited enabled server locally or restore inherited enabled behavior, but may not rewrite the upstream connection definition.
- Workspace `/agents` must extend internal-agent policy so MCP access can be `disabled`, `all_enabled_servers`, or `selected_servers`; when selected mode is active, assignments persist by server rather than by remote discovered tool.
- Internal-agent runtime preparation must resolve effective platform servers through current workspace policy and current internal-agent MCP policy, then discover and materialize callable runtime tools named `mcp.<server_code>.<tool_name>` for the current run only.

## Scope Boundary

This feature covers dedicated persistence, authenticated management APIs, frontend settings and workspace pages, internal-agent MCP policy, runtime discovery and materialization, focused tests, and browser verification for the bounded first version.

## Assumptions

- V1 keeps platform ownership of all MCP server connection definitions; workspace-local server authoring and parent override are deferred.
- V1 extends the current internal-agent policy model rather than the system-agent registry rows; MCP-to-agent attachment remains a parallel API family.
- V1 remains auth-only for management APIs; it does not add route-level permission enforcement beyond the current authenticated boundary.
- V1 reuses current shared HTTP client profiles and does not introduce per-server timeout or pool tuning.
- V1 keeps `disable` and `delete` semantics distinct:
  - `disable` preserves references and removes future effective availability
  - `delete` cascades dependent workspace overrides, selected-server assignments, and connection-pool cache entries
- V1 keeps current run snapshots stable; only future runs observe later MCP configuration changes.

## In Scope

- platform MCP gateway singleton config for `/mcp/variables`
- platform-owned remote MCP server registry
- platform server health-check and latest health summary behavior
- workspace-effective MCP server list for the active workspace with local enable or disable override behavior only
- internal-agent MCP access mode and selected-server assignments
- runtime discovery and materialization of effective MCP tools
- management and runtime error handling for deleted selected servers, platform-disabled servers, and remote discovery failure
- backend tests for persistence, policy resolution, delete cascade, and runtime materialization
- frontend and browser proof for `/settings/mcp-servers`, `/mcp-servers`, and `/agents`

## Out Of Scope

- workspace-local MCP server creation
- workspace override of inherited `url`, `headers`, or `auth`
- stdio transport
- permission-matrix enforcement
- per-server timeout, keepalive, pool, or session-limit tuning
- secret-vault integration
- per-tool MCP assignment
- inbound MCP endpoints other than `/mcp/variables`

## Constraints

- All management API responses must keep the repo's uniform envelope.
- Platform settings must reuse the existing settings-shell route and navigation pattern.
- Workspace MCP server management must reuse the current authenticated shell workspace context.
- Internal-agent MCP access must remain parallel to the existing system-agent API family rather than mutating system-agent rows.
- Runtime federation must satisfy the current execution gate:
  - every callable MCP tool must be both registered for execution and materialized into the run context
  - unauthorized or missing tool names must continue to fail through the existing runtime guardrails
- Management APIs may rely on current authenticated access only; they must not silently imply a new permission model.

## Feature Model

Bounded context:

- platform MCP gateway and registry
- workspace-effective MCP server policy
- internal-agent MCP server access
- runtime MCP tool federation

Entities:

- `PlatformMcpGatewayConfig`: singleton config for inbound variables MCP exposure
- `PlatformMcpServer`: platform-owned remote MCP server definition with connection metadata and latest health summary
- `WorkspaceMcpServerPolicy`: current-workspace effective-state override for an inherited platform server
- `InternalAgentMcpAccessMode`: one of `disabled`, `all_enabled_servers`, `selected_servers`
- `WorkspaceInternalAgentMcpServerAssignment`: selected-server binding keyed by `(workspaceCode, agentCode, serverCode)`
- `RuntimeMcpToolRecord`: current-run materialized MCP tool named `mcp.<server_code>.<tool_name>`

Policies:

- platform owns the canonical remote server definition in v1
- workspace may only change current-workspace effective enabled state for inherited platform servers
- platform disable dominates every workspace
- internal-agent selected-server assignments may only reference servers currently effective in the workspace
- deleting a platform server cascades workspace effective-state overrides, selected-server assignments, and cached pool state
- active runs keep their tool snapshot even if MCP configuration changes later

## Page And Module Surface

Platform settings:

- route surface: `/settings/mcp-servers`
- behavior:
  - render an MCP gateway card for `/mcp/variables`
  - render a platform MCP server registry with create, edit, disable, enable, health, and delete controls

Workspace MCP servers:

- route surface: `/mcp-servers`
- behavior:
  - render only inherited platform servers for the active workspace
  - show source ownership, platform enabled state, and current-workspace effective state
  - allow only local disable or restore-inherited-enable actions

Workspace agents:

- route surface: `/agents`
- behavior:
  - extend the internal-agent section with MCP access mode and selected-server assignment
  - keep the system-agent section behavior unchanged

Backend modules:

- new namespace under `tron-backend/src/application/mcp/**` for gateway, registry, policy, runtime federation, and health flows
- new inbound routes under `tron-backend/src/adapters/inbound/http/mcp/**`
- internal-agent MCP policy extensions under existing `tron-backend/src/application/agent/**` and `tron-backend/src/adapters/inbound/http/agents/**`
- persistence additions under `tron-backend/src/adapters/outbound/persistence/sqlite/**`

Frontend modules:

- new domain namespace under `tron-frontend/src/domain/mcp/**`
- settings navigation and page registration updates
- workspace MCP page under the main shell
- internal-agent page extension under `tron-frontend/src/domain/agent/**`

## Repository Findings

- `tool-registry` explicitly scoped `external plugin/MCP tool federation` out of that feature, so `mcp-management` must be a new bounded workspace rather than a silent extension of `tool-registry`.
- Current auth middleware exposes authenticated user and locale only; protected routes are globally guarded by `require_auth`, and no route-level `MCP` permission checks were found in the current backend request path.
- Current internal-agent policy is already modeled as a parallel API family distinct from workspace system-agent rows; MCP-to-agent assignment should remain parallel in the same spirit.
- Current runtime invocation only executes tools that are both registered and present in `context.tools`, so runtime MCP federation must update both execution registration and run-context materialization.
- Current shared HTTP client configuration provides `default`, `modelInference`, and `runtimeControl` profiles with connect-timeout and pool settings only; MCP health and remote discovery should align with that foundation.
- Frontend protected and localized CRUD requests should use `requestApi`, not `useServiceQuery`, because MCP management pages need both bearer auth and `Accept-Language`.
- Existing frontend shells already cover the expected UI shapes:
  - settings-shell registry page patterns
  - workspace split-pane or table patterns
  - internal-agent policy editor patterns

## Open Decisions

- None. The first-version boundary is intentionally narrowed to avoid unresolved local-authoring, permission, and per-tool policy ambiguity.
