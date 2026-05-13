# Agent Report: red-all-status-include-archived

## Agent Identity

- agent_id: test
- role: TEST RED worker
- task: TEST-RED-17
- branch: feature/clouds3n/user-management-model/test/red-all-status-include-archived
- worktree: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-all-status-include-archived

## Task Results

| task id | result | notes |
| --- | --- | --- |
| TEST-RED-17 | fail_expected_red | Added frontend Playwright coverage for `/admin/users` All Status behavior. The test proves the current UI/API adapter does not use the approved all-status list contract. |

## TDD Evidence

- acceptance IDs: AC-UM-11, AC-UM-01, AC-UM-08
- TDD IDs: TDD-UM-09, TDD-UM-15
- expected RED reason: selecting `All Status` currently sends `status=` instead of omitting `status`, does not send `includeArchived=true`, and therefore does not render an Archived row from the intercepted include-archived response path.
- observed failure:
  - `expect.soft(allStatusRequest.searchParams).not.toHaveProperty("status")` failed with received value `""`.
  - `expect.soft(allStatusRequest.searchParams.includeArchived).toBe("true")` failed with received value `undefined`.
  - `All Status Archived` was not visible.

## Changed Paths

- tron-frontend/tests/user-management-smoke.spec.ts
- docs/features/user-management-model/reports/agents/red-all-status-include-archived.md

## Commands

| command | result |
| --- | --- |
| `rtk make fe-install` | pass; installed frontend dependencies needed for Playwright in this worker worktree. |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "all status includeArchived" --reporter=line` | fail_expected_red; 1 failed. Behavior-level failure: actual All Status request includes `status=""`, omits `includeArchived=true`, and the Archived row is not rendered. |

## Blockers

- None.
