# Backend Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: in_progress
- execution_mode: parallel

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY.

## Task Rows

- AC-TOOL-01 | phase: tdd | owner: backend-worker | scope: code-backed tool catalog domains, concrete tools, and read APIs in `tron-backend/src/application/tool_registry/**` plus HTTP route wiring and backend tests | command_ref: CMD-TOOL-BE-TEST | status: in_progress.
- AC-TOOL-02 | phase: tdd | owner: backend-worker | scope: internal-agent policy persistence, workspace-effective resolution, and workspace internal-agent APIs | command_ref: CMD-TOOL-BE-TEST | status: in_progress.
- AC-TOOL-03 | phase: tdd | owner: lead | scope: runtime invocation integration, effective assigned-tool derivation, and domain-discovery materialization seam | command_ref: CMD-TOOL-BE-TEST | status: in_progress.

## Worktree Notes

- Lead branch: `feature/clouds3n/tool-registry/base`.
- Backend worker runtime id: `019e85d1-2108-7930-aaae-3534485edc60`.

## Integration Notes

- Catalog and policy should avoid ambiguous naming with existing runtime executor `ToolRegistry`.
- Main-agent invocation must derive effective tools when request tools are absent.

## Open Decisions

- None.
