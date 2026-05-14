# Feature Evolution Log: User Management Model

## Summary

- feature_slug: user-management-model
- total_changes: 4
- first_evolution: EV-04
- last_evolution: EV-07

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

### EV-05

- change_id: EV-05
- timestamp: 2026-05-13
- trigger: bug_discovery
- change_type: task_execution_change, documentation_only
- gate_reopened: Phase 3
- diff_summary: |
    What changed vs before:
    - REVIEW-04 found `All Status` selected the directory list with `status=` and no explicit archive inclusion intent.
    + Add a focused RED/GREEN/VERIFY follow-up so `All Status` omits `status`, sends `includeArchived=true`, and renders Active plus Archived rows through the approved list contract.
- downstream_impact: tasks/test-task.md, tasks/frontend-task.md, tasks/review-task.md, orchestration/worktree-plan.md, orchestration/integration-log.md, reports/test-report.md, reports/frontend-result.md, reports/review-report.md
- old_evidence_status: retained for existing statistics/create/layout behavior, superseded only for All Status frontend query evidence
- user_approval: approved_with_defaults from 2026-05-13 continue after REVIEW-04 finding

### EV-06

- change_id: EV-06
- timestamp: 2026-05-13
- trigger: user_request
- change_type: requirement_expansion, acceptance_change, test_scope_change, task_execution_change
- gate_reopened: Phase 1A paused before downstream user-management edits
- diff_summary: |
    What changed vs before:
    - User-management interaction feedback and long-running Export/Save loading behavior were not part of the closed EV-05 scope.
    + User requested long-running Export/Save loading animation and required all user interaction feedback to use a UI-framework Toast/Snackbar bubble floating top-right with auto-dismiss. Repository scan found no unified wrapper, so user-management is paused and new feature `shared-ui-feedback` is opened first.
- downstream_impact: user-management spec/test/task changes are deferred until `docs/features/shared-ui-feedback` delivers the shared feedback primitive
- old_evidence_status: retained; no user-management production code or tests changed for EV-06
- user_approval: approved_with_defaults from 2026-05-13 user instruction to pause current feature and create a new feature when no unified wrapper exists

### EV-07

- change_id: EV-07
- timestamp: 2026-05-14
- trigger: user_request
- change_type: requirement_expansion, acceptance_change, test_scope_change, task_execution_change
- gate_reopened: Phase 1A
- diff_summary: |
    What changed vs before:
    - EV-06 paused user-management feedback/loading work because no shared Toast/Snackbar primitive existed in the base branch.
    + `master` now contains the completed `shared-ui-feedback` feature; user-management evaluation resumes against that canonical master version and must migrate interaction feedback to `useToast` while adding explicit loading states for long-running Export and Save actions.
- downstream_impact: spec/feature-spec.md, spec/acceptance.md, spec/test-scope.md, spec/test-plan.md; downstream task/worktree rows are deferred until this reopened test plan is accepted
- old_evidence_status: retained for EV-05 user-management behavior; superseded only for feedback surface and async loading evidence
- user_approval: approved_with_defaults on 2026-05-14; user authorized autonomous continuation until completion

## Downstream Artifact Migration

- EV-04 adds replacement RED rows `TEST-RED-15` and `TEST-RED-16`, GREEN rows `BE-GREEN-05` and `FE-GREEN-03`, VERIFY row `TEST-VERIFY-02`, and REVIEW row `REVIEW-04`.
- EV-05 adds follow-up RED row `TEST-RED-17`, GREEN row `FE-FIX-03`, VERIFY row `TEST-VERIFY-03`, and REVIEW row `REVIEW-05` for the `All Status` include-archived contract.
- EV-06 creates the separate prerequisite feature workspace `docs/features/shared-ui-feedback` and intentionally adds no user-management task rows until that prerequisite is completed.
- EV-07 adds TDD cycles `TDD-UM-19` and `TDD-UM-20`, then proceeds to RED/GREEN/VERIFY/REVIEW task rows after user approval.
- Prior evidence for `TDD-UM-01..13` remains retained unless the changed behavior explicitly supersedes current-page-derived statistics or inline create ownership.

## Residual Risks

- User-management loading animation and Toast/Snackbar migration remain unevaluated in worker evidence until EV-07 RED/GREEN/VERIFY rows are approved and executed.
