# Feature Evolution Log: User Management Model

## Summary

- feature_slug: user-management-model
- total_changes: 1
- first_evolution: EV-04
- last_evolution: EV-04

## Change History

Record every evolution event. `spec/feature-spec.md` remains the current authoritative requirement document; this log preserves the historical trail only.

### EV-04

- change_id: EV-04
- timestamp: 2026-05-13
- trigger: user_request
- change_type: requirement_expansion, api_contract_change, acceptance_change, test_scope_change, task_execution_change
- gate_reopened: Phase 1A
- diff_summary: |
    What changed vs before:
    - User dashboard cards derived values from current directory records, Add User behavior lived inline on `/admin/users`, and list/count consistency was only partially specified.
    + User dashboard cards must call four dedicated statistic APIs, list/search totals must match repository counts, deterministic seeds must prove pagination/count behavior, directory columns must separate display name and contact, and create behavior moves to `/admin/users/new`.
- downstream_impact: spec/feature-spec.md, spec/api-contract.md, spec/acceptance.md, spec/test-scope.md, spec/test-plan.md, tasks, worktrees, reports
- old_evidence_status: retained for prior behavior, superseded for statistics, list/count consistency, directory layout, and create sub-page behavior
- user_approval: approved_with_defaults from 2026-05-13 user-provided refactor plan

## Downstream Artifact Migration

- EV-04 adds replacement RED rows `TEST-RED-15` and `TEST-RED-16`, GREEN rows `BE-GREEN-05` and `FE-GREEN-03`, VERIFY row `TEST-VERIFY-02`, and REVIEW row `REVIEW-04`.
- Prior evidence for `TDD-UM-01..13` remains retained unless the changed behavior explicitly supersedes current-page-derived statistics or inline create ownership.

## Residual Risks

- Browser evidence for EV-04 is pending until the RED/GREEN/VERIFY worker sequence completes.
