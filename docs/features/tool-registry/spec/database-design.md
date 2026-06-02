# Database Design: Tool Registry

## Database Decision

- status: approved

Tool definitions remain code-backed and are not persisted as mutable rows.

Persistence is limited to internal-agent policy and explicit workspace assignments.

## Existing Schema Impact

Current agent-management storage supports:

- platform main-agent config
- system-agent registry rows
- workspace system-agent policies

This feature adds internal-agent policy storage in parallel.

## Proposed Schema

- `workspace_internal_agent_policies`
  - `workspace_code`
  - `agent_code`
  - `tool_assignment_mode`
  - `tool_exposure_mode`
  - `expanded_domain_persistence_mode`
  - timestamps
- `workspace_internal_agent_tool_assignments`
  - `workspace_code`
  - `agent_code`
  - `tool_code`
  - timestamps

Optional future platform defaults storage may be deferred in v1 because `main-agent` defaults can remain code-backed while only workspace overrides are persisted.

## Relationships And Cardinality

- one workspace can have zero or more internal-agent policy override rows
- one workspace internal-agent pair can have zero or more explicit selected-tool assignment rows

## Migration Plan

- add new migration for internal-agent policy and assignment tables
- preserve existing agent-management tables and data

## Query And Performance Plan

- lookups are keyed by `workspace_code` and `agent_code`
- assignment queries should be ordered by `tool_code`
- no pagination is required for assignments in v1

## Data Governance

- stale selected-tool rows must not break runtime; they should be ignored at effective-resolution time and surfaced to operators when feasible

## Test Data And Fixtures

- integration tests should seed a workspace, `main-agent` default policy, and representative tool definitions from the code-backed catalog
- explicit selected-tool fixtures should include both valid tool codes and one stale tool-code scenario

## Validation Commands

- `rtk make be-test`
