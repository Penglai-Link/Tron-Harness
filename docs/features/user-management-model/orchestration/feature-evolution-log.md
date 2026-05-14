# Feature Evolution Log: User Management Model

## Summary

- feature_slug: user-management-model
- total_changes: 5
- first_evolution: EV-04
- last_evolution: EV-08

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

### EV-08

- change_id: EV-08
- timestamp: 2026-05-14
- trigger: user_request
- change_type: requirement_expansion, acceptance_change, test_scope_change, task_execution_change
- gate_reopened: Phase 1A
- diff_summary: |
    What changed vs before:
    - Directory search still dispatched per keystroke, the row action button was placeholder-only, detail editing lived in a detached edit form, and Add User still required manual `userCode` entry.
    + Directory search must debounce 300ms to 500ms, skip empty-keyword search requests, and ignore stale responses; the directory row action menu must expose Archive or Restore by status; User Detail personal information must be easier to scan and edit inline while `userCode` and `username` stay read-only; Add User must rely on backend-generated `userCode`.
- downstream_impact: spec/feature-spec.md, spec/api-contract.md, spec/acceptance.md, spec/test-scope.md, spec/test-plan.md, tasks/test-task.md, tasks/frontend-task.md, tasks/review-task.md, orchestration/worktree-plan.md, orchestration/integration-log.md, reports/test-report.md, reports/frontend-result.md, reports/review-report.md
- old_evidence_status: retained for EV-07 feedback/loading behavior, superseded only for eager search dispatch, placeholder row actions, detached detail editing, and manual create `userCode`
- user_approval: approved_with_defaults from 2026-05-14 direct intent list for feature evaluation and implementation

### EV-09

- change_id: EV-09
- timestamp: 2026-05-14
- trigger: user_request
- change_type: requirement_clarification, acceptance_change, test_scope_change, implementation_alignment
- gate_reopened: Phase 1A
- diff_summary: |
    What changed vs before:
    - EV-08 closed debounced search, row actions, inline edit, and generated-code create flow, but runtime review still found detail/create field gaps and archived-detail routing drift.
    + User Detail must show `Phone Country Code` separately, allow editing of `Phone Country Code`, `Phone`, `Locale`, and `Timezone`, visibly emphasize read-only `userCode` and `username`, refresh `Updated` after profile/lifecycle changes, and load archived users through the direct detail endpoint. Create User must drop Role/Space inputs, align its field set to User Detail personal information, and auto-fill `Display Name` from `First Name` plus `Last Name` by default.
- downstream_impact: `spec/acceptance.md`, `spec/test-plan.md`, `orchestration/integration-log.md`, `reports/test-report.md`, `reports/frontend-result.md`, `reports/review-report.md`, `reports/worktree-report.md`
- old_evidence_status: retained for EV-08 search/row-action/generated-code behavior, superseded only for detail field parity, read-only styling emphasis, archived-detail routing, and create-form field parity
- user_approval: approved_with_defaults from 2026-05-14 runtime review checklist and direct adjustment request

## Downstream Artifact Migration

- EV-04 adds replacement RED rows `TEST-RED-15` and `TEST-RED-16`, GREEN rows `BE-GREEN-05` and `FE-GREEN-03`, VERIFY row `TEST-VERIFY-02`, and REVIEW row `REVIEW-04`.
- EV-05 adds follow-up RED row `TEST-RED-17`, GREEN row `FE-FIX-03`, VERIFY row `TEST-VERIFY-03`, and REVIEW row `REVIEW-05` for the `All Status` include-archived contract.
- EV-06 creates the separate prerequisite feature workspace `docs/features/shared-ui-feedback` and intentionally adds no user-management task rows until that prerequisite is completed.
- EV-07 adds TDD cycles `TDD-UM-19` and `TDD-UM-20`, then proceeds to RED/GREEN/VERIFY/REVIEW task rows after user approval.
- EV-08 adds TDD cycles `TDD-UM-21` through `TDD-UM-24`; EV-09 keeps the same focused operator-refinement command surface while extending runtime assertions for archived-detail routing, detail/create field parity, and `Updated` refresh behavior on base.
- Prior evidence for `TDD-UM-01..13` remains retained unless the changed behavior explicitly supersedes current-page-derived statistics or inline create ownership.

## Residual Risks

- EV-07 shared Toast/Snackbar migration and action-local loading evidence is complete through RED, GREEN, TEST-FIX, VERIFY, and REVIEW worker rows. Secondary role/space/lifecycle/reset/copy pending states were source-reviewed; focused delayed runtime loading evidence covers Export, Create User, and Save Profile.
- EV-08 directory/detail/create refinement is no longer paused; focused operator-refinement evidence was extended on base to cover archived-detail routing plus detail/create field parity, and full `user-management-smoke` now passes `30/30`.
