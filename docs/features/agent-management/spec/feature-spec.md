# Feature Spec: Agent Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: agent-management
- feature_name: Agent Management
- status: approved_with_defaults
- owner: clouds3n
- source_request: complete the approved agent-management PRD so Tron can persist one platform `Main Agent`, scan supported system coding agents, expose platform and workspace control surfaces, and make the saved main-agent configuration affect runtime behavior.
- main_agent: Codex lead agent
- dependency_features: `workspace-management`, `agent-invocation-api`, `shared-ui-feedback`
- dependency_contracts: authenticated settings shell, authenticated workspace shell, uniform API response envelope, active-workspace validation, existing agent invocation path

## Product Source Slices

- PRD module and feature slices:
  - `Feature: Agent Management - Platform Main Agent Configuration`
  - `Feature: Agent Management - System Agent Registry`
  - `Feature: Agent Management - Workspace Effective System Agents`
  - `Feature: Agent Management - Runtime Main Agent Consumption`
- PRD page and interaction slices:
  - `Page: /settings/agents - Agent Settings`
  - `Page: /agents - Workspace Agent Management`
  - `Interaction: /settings/agents - Save Main Agent`
  - `Interaction: /settings/agents - Scan System Agents`
  - `Interaction: /settings/agents - Toggle Platform System Agent`
  - `Interaction: /agents - Toggle Workspace System Agent`
- PRD rule and exception slices:
  - `Rule: Agent Management - Main Agent Scope`
  - `Rule: Agent Management - System Agent Discovery`
  - `Rule: Agent Management - Platform Agent Policy`
  - `Rule: Agent Management - Workspace Override Semantics`
  - `Rule: Agent Management - Unavailable Agent Status`
  - `Rule: Agent Management - Main Agent Runtime Consumption`
  - `Exception: Agent Management - Main Agent Missing Config`
  - `Exception: Agent Management - Agent Unavailable During Action`
  - `Exception: Agent Management - Missing Workspace`

## Requirement

Tron needs one product-level agent-management feature set with two operator surfaces and one runtime effect.

- Platform settings at `/settings/agents` must let an admin edit the singleton `Main Agent` fields `personality` and `instructions`, scan supported system coding agents such as `codex` and `opencode`, and globally enable or disable discovered system agents.
- Workspace management at `/agents` must show only inherited system-agent rows from platform scope. It must not show the `Main Agent`. Workspace operators may only disable one inherited system agent locally or enable it back to platform behavior.
- Backend startup and manual rescan must probe the supported system-agent set, preserve known rows, mark missing agents as `Unavailable`, and keep platform policy intact.
- The saved platform `Main Agent` config must be consumed by the existing main-agent runtime path so the settings page changes real behavior instead of storing dead data.

## Scope Boundary

This feature covers new backend persistence, registry scanning, policy resolution, HTTP endpoints, frontend settings and workspace pages, mock and API wiring, focused tests, and browser verification for agent management.

## Assumptions

- Supported system-agent discovery in v1 is a bounded backend probe over known executables, not open-ended plugin authoring.
- System agents remain read-only metadata rows from the product perspective; only enable or disable policy is mutable.
- Workspace-effective system-agent behavior can be modeled as platform baseline plus current-workspace local disable overrides.
- Main-agent runtime consumption applies to main-agent style invocations and does not redesign task-agent routing.

## In Scope

- Persist one platform `Main Agent` singleton configuration.
- Add startup scan plus manual rescan for supported system agents.
- Persist global system-agent availability facts and global enable or disable policy.
- Persist workspace-local system-agent disable overrides.
- Expose `/settings/agents` and `/agents`.
- Hide the platform `Main Agent` from workspace agent management.
- Add backend tests for config persistence, scan policy, workspace-effective state, and runtime consumption.
- Add frontend smoke coverage for settings and workspace agent-management flows.

## Out Of Scope

- Per-workspace main-agent authoring.
- System-agent archive lifecycle.
- Custom system-agent authoring or editing.
- Writing external Codex or OpenCode config files.
- Deep runtime routing differences per system agent.

## Constraints

- All API responses must keep the repo's uniform envelope.
- Platform settings must reuse the existing settings-shell route and navigation pattern.
- Workspace agent management must reuse the current authenticated shell workspace context.
- `Main Agent` stays platform-only and hidden from workspace agent tables.
- System-agent rows cannot be archived in v1.

## Feature Model

Bounded context:

- Platform agent configuration and workspace-effective system-agent policy.

Entities:

- `PlatformMainAgentConfig`: singleton config storing `personality` and `instructions`.
- `SystemAgentRegistryRow`: known system-agent row keyed by `agentCode`, with discovery metadata, platform enabled flag, and availability state.
- `WorkspaceSystemAgentPolicy`: current-workspace policy override keyed by `workspaceCode` and `agentCode`.

Policies:

- `Main Agent` exists once at platform scope and is not represented as a workspace-managed row.
- Supported system-agent discovery updates known rows in place and marks missing rows `Unavailable` instead of deleting them.
- Platform disable dominates all workspace-local state.
- A workspace-local `Enable` action removes the local disable override and returns the row to platform behavior.
- Unavailable system-agent rows are read-only on both platform and workspace surfaces.

## Page And Module Surface

Platform settings:

- Route surface: `/settings/agents`
- Behavior:
  - render a `Main Agent` editor card with only `Personality` and `Instructions`
  - render a lower system-agent registry table with `Scan`
  - show system-agent `Status` as `Enabled`, `Disabled`, or `Unavailable`
  - allow only `Enable` or `Disable` row actions for system agents

Workspace agent management:

- Route surface: `/agents`
- Behavior:
  - render only inherited system-agent rows
  - exclude the `Main Agent`
  - render `Inherited From = Platform`
  - allow only `Enable` or `Disable` actions for the current workspace

Backend runtime:

- Modules: existing `application::agent` runtime and invocation flow
- Behavior:
  - load the persisted platform main-agent config during main-agent invocation preparation
  - prepend or otherwise inject the saved main-agent guidance into the runtime request so saved values affect actual model input

## Repository Findings

- The backend already has a natural namespace at `tron-backend/src/application/agent` plus `/api/v1/agents/*` routing for new management endpoints.
- Skill registry code already demonstrates startup or manual rescan patterns, workspace-effective inheritance, and route structure worth copying for system-agent management.
- The frontend already has the exact platform shell pattern in `SettingsShell` and the closest split-pane or table patterns in the skill and model pages.
- The existing `/agent` page is a mock chat surface and should remain separate from the new `/agents` workspace management route.

## Open Decisions

- None.
