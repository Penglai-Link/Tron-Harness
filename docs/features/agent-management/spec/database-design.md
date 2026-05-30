# Database Design: Agent Management

## Database Decision

- status: approved
- Add dedicated persistence for platform main-agent config, system-agent registry state, and workspace system-agent overrides.
- Keep v1 semantics latest-only and do not introduce archive tables.

## Existing Schema Impact

- Current backend persistence covers agent run history only and does not store operator-facing agent-management data.
- Existing workspace validation and lineage logic can be reused without changing workspace schema.

## Proposed Schema

- `platform_main_agent_configs`
  - singleton row keyed by stable id or scope
  - fields: `personality`, `instructions`, `created_at`, `updated_at`
- `system_agent_registries`
  - one row per supported `agentCode`
  - fields: `agent_code`, `name`, `description`, `command_name`, `source`, `enabled`, `available`, `last_scanned_at`, `last_seen_at`, `last_error`
- `workspace_system_agent_policies`
  - one row per current-workspace override
  - fields: `workspace_code`, `agent_code`, `enabled`, `created_at`, `updated_at`

## Relationships And Cardinality

- One platform main-agent config row exists for the whole product.
- One known system-agent registry row exists per supported system agent.
- One workspace may own at most one override row per known system agent.
- Many workspaces may reference the same system-agent registry row through effective-state resolution.

## Migration Plan

- Add idempotent migrations for the three tables above.
- Bootstrap the platform main-agent singleton lazily on first read or first save.
- Run supported system-agent scan during application startup after repositories are available.
- Preserve known registry rows during rescan and update `available` instead of deleting rows when discovery fails.

## Query And Performance Plan

- Platform list reads only the system-agent registry table and derives `status` from `enabled` and `available`.
- Workspace-effective list joins platform registry rows with current-workspace override rows.
- Runtime main-agent consumption performs one lightweight singleton config read during main-agent invocation preparation.

## Data Governance

- System-agent rows store metadata and policy only; no external tool secrets are persisted here.
- Workspace overrides are local policy facts and must not rewrite platform baseline state.
- `last_error` is diagnostic only and may be surfaced cautiously in backend logs or future admin views.

## Test Data And Fixtures

- Backend tests should cover at least `codex` and `opencode` probe outcomes.
- Fixtures should cover one workspace-local disable override and one unavailable system-agent row.
- Runtime tests should prove main-agent config absence fallback and saved-config consumption.

## Validation Commands

- `CMD-BE-CHECK`
- `CMD-AGENT-BE-TEST`
