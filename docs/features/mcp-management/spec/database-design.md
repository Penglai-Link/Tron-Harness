# Database Design: MCP Management

## Database Decision

- status: approved
- impact: yes
- approved_by: clouds3n
- approved_at: 2026-06-05

This feature adds dedicated persistence for the platform MCP gateway, platform MCP server registry, workspace-effective MCP server overrides, and selected-server internal-agent assignments. It also extends the existing internal-agent policy record with one new MCP access mode field.

## Existing Schema Impact

Current backend persistence already stores internal-agent policy, workspace validation, authenticated user state, and runtime history, but it does not store productized MCP registry data.

Impacted existing paths:

- `workspace_internal_agent_policies` must gain `mcp_access_mode`.
- current internal-agent policy repository and DTO mapping must include the new field.
- current runtime preparation will depend on persisted platform MCP server definitions plus workspace and internal-agent policy reads.
- current SQLite migration registration must be updated because this repo hard-codes migration constants and apply guards.

## Proposed Schema

### Table: platform_mcp_gateway_configs

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| singleton_key | TEXT | yes | primary key | Fixed singleton identifier. |
| gateway_enabled | INTEGER | yes | boolean | Whether the platform MCP gateway is exposed. |
| variables_endpoint_enabled | INTEGER | yes | boolean | Whether `/mcp/variables` is exposed when the gateway is enabled. |
| created_at | INTEGER | yes | unix epoch milliseconds | Creation timestamp. |
| updated_at | INTEGER | yes | unix epoch milliseconds | Last update timestamp. |

### Table: platform_mcp_servers

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| server_code | TEXT | yes | primary key | Stable server identifier used across policies, assignments, and runtime naming. |
| name | TEXT | yes | none | Operator-facing display name. |
| description | TEXT | no | none | Optional operator description. |
| url | TEXT | yes | none | Remote MCP transport URL. |
| auth_kind | TEXT | yes | validated in application layer | Connection auth strategy. |
| auth_payload_json | TEXT | no | valid JSON when present | Opaque auth payload. |
| headers_json | TEXT | no | valid JSON when present | Opaque header map. |
| enabled | INTEGER | yes | boolean | Platform enabled state. |
| last_health_status | TEXT | yes | check in `never_checked`, `healthy`, `unhealthy` | Latest explicit health result. |
| last_health_message | TEXT | no | none | Latest explicit health summary or error. |
| last_health_checked_at | INTEGER | no | unix epoch milliseconds | Latest explicit health-check timestamp. |
| created_at | INTEGER | yes | unix epoch milliseconds | Creation timestamp. |
| updated_at | INTEGER | yes | unix epoch milliseconds | Last update timestamp. |

Indexes and constraints:

- `uk_platform_mcp_servers_server_code` on `server_code`.
- `idx_platform_mcp_servers_enabled_updated_at` on `enabled`, `updated_at`.

### Table: workspace_mcp_server_policies

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| workspace_code | TEXT | yes | composite primary key | Workspace tenancy boundary. |
| server_code | TEXT | yes | composite primary key | Referenced platform server identifier. |
| enabled | INTEGER | yes | boolean | Local effective-state override. |
| created_at | INTEGER | yes | unix epoch milliseconds | Creation timestamp. |
| updated_at | INTEGER | yes | unix epoch milliseconds | Last update timestamp. |

Indexes and constraints:

- primary key on `workspace_code`, `server_code`.
- `server_code` references `platform_mcp_servers.server_code` with delete cascade.

### Changed Table: workspace_internal_agent_policies

| field | change_type | notes |
| --- | --- | --- |
| `mcp_access_mode` | add column | `TEXT NOT NULL DEFAULT 'disabled'`; allowed values `disabled`, `all_enabled_servers`, `selected_servers`. |

### Table: workspace_internal_agent_mcp_server_assignments

| field | SQLite type | required | constraints | description |
| --- | --- | --- | --- | --- |
| workspace_code | TEXT | yes | composite primary key | Workspace tenancy boundary. |
| agent_code | TEXT | yes | composite primary key | Internal-agent identifier. |
| server_code | TEXT | yes | composite primary key | Selected platform server identifier. |
| created_at | INTEGER | yes | unix epoch milliseconds | Creation timestamp. |
| updated_at | INTEGER | yes | unix epoch milliseconds | Last update timestamp. |

Indexes and constraints:

- primary key on `workspace_code`, `agent_code`, `server_code`.
- `server_code` references `platform_mcp_servers.server_code` with delete cascade.

## Relationships And Cardinality

- One platform gateway singleton row exists for the whole product.
- One platform MCP server row exists per `server_code`.
- One workspace may own at most one effective-state override row per platform MCP server.
- One workspace internal-agent policy row may reference many selected MCP servers.
- Many workspace override rows and many selected-server assignment rows may reference the same platform MCP server.

Domain invariants:

- v1 platform owns the canonical server definition.
- workspace rows never copy `url`, `auth`, or `headers`.
- internal-agent selected-server rows persist by server, not by discovered tool.
- platform delete cascades server-dependent rows; platform disable preserves them.

## Migration Plan

- add one migration for `platform_mcp_gateway_configs`
- add one migration for `platform_mcp_servers`
- add one migration for `workspace_mcp_server_policies`
- add one migration for `workspace_internal_agent_mcp_server_assignments`
- add one migration altering `workspace_internal_agent_policies` to append `mcp_access_mode`
- update `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs` with every new migration constant and apply guard

Operational notes:

- bootstrap the gateway singleton lazily on first read or first save
- default `mcp_access_mode` to `disabled` so external MCP access is opt-in
- keep current run-context snapshots in memory only; no runtime migration is needed for already-finished runs

## Query And Performance Plan

Primary query paths:

- platform gateway singleton read and write
- platform MCP server list, detail, health update, and lifecycle actions
- active-workspace effective MCP server list
- internal-agent MCP access-mode read and selected-server replacement
- runtime resolution of effective selected server codes before discovery

Performance notes:

- workspace effective list is a platform registry projection plus current-workspace override rows only; v1 does not require full multi-level lineage traversal because workspace-local server authoring is deferred
- runtime resolution should key cache entries by `server_code` so disable/delete invalidation remains precise
- MCP health checks and runtime discovery should reuse current shared HTTP client profiles instead of creating row-level timeout or pool metadata

## Data Governance

- `server_code` is the business key for policy, assignment, cache invalidation, and runtime tool naming.
- `url`, `auth_payload_json`, and `headers_json` remain opaque connection metadata from the database perspective; semantic validation belongs to the application layer.
- latest health summary is operator-facing registry metadata only and must not be treated as runtime authorization state.
- v1 remains auth-only; no schema changes are introduced for route-level permission grants.
- stale selected-server submissions must be rejected at the application layer even though delete cascade should keep the database clean of dangling assignments.

## Test Data And Fixtures

Backend fixtures should cover at least:

- one enabled platform MCP server
- one disabled platform MCP server
- one explicit health success result
- one explicit health failure result
- one workspace-local disable override
- one internal-agent selected-server assignment set
- one delete-cascade scenario that removes workspace overrides and selected-server assignments together
- one runtime-preparation scenario with two runs that observe different MCP tool snapshots before and after a config mutation

## Validation Commands

- `rtk make be-check`
- `rtk make be-test`
- focused reruns should use direct `cargo test --manifest-path tron-backend/Cargo.toml ...` after the required Makefile entrypoint when a specific MCP test target needs iteration
