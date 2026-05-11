# Feature Evolution Workflow

Use this reference when an existing `docs/features/<feature-slug>/` workspace must change after initial drafting, task planning, active implementation, or delivery.

## Orientation And Freeze

Read existing `spec/`, `tasks/`, `orchestration/`, `reports/`, active git worktrees, and sibling feature specs under the feature root before editing. Scan sibling `*/spec/feature-spec.md` files, excluding the current feature, to identify strong feature dependencies from feature spec content. If strong dependencies exist, carry the dependency feature names into the current `spec/feature-spec.md`, impact classification, downstream artifact updates, and worker context. If work is in progress, pause new worker dispatch and merges until impact analysis is confirmed. Preserve files and update in place.

Classify the change as one or more of:

- requirement expansion, contraction, or clarification
- domain model or page/module boundary refactor
- database or migration change
- API/DTO/permission/error contract change
- acceptance criteria or test scope change
- task/worktree execution change
- documentation or evidence correction only

Record classification, user request, affected files, and pause/resume decision in `orchestration/integration-log.md` or affected spec/report sections.

## Reopen The Earliest Affected Gate

Use the earliest affected gate, then re-check every downstream artifact that depends on it:

| change affects | reopen from | downstream artifacts to re-check |
| --- | --- | --- |
| requirement, roles, pages, domain model, module boundary | Workflow A Phase 1A | database, API, acceptance, test scope, test plan, tasks |
| persistence, schema, migration, data governance, query behavior | Workflow A Phase 1B | API, acceptance, test scope, test plan, tasks |
| endpoint, DTO, parameter passing, auth, error, adapter boundary | Workflow A Phase 1C | acceptance, test scope, test plan, tasks |
| acceptance IDs, evidence expectations, test boundary | Workflow A Phase 1D | test plan, tasks |
| TDD mapping only | Workflow A Phase 2 | tasks |
| implementation sequencing or ownership only | Workflow A Phase 3 | worktrees and reports |

Run the matching `--spec-stage` validation after each reopened gate. Run `--complete-spec` whenever acceptance, test scope, or test plan changes. If `spec/test-plan.md` changes, reset `Test Plan Decision` to `waiting_human_decision` and stop before task migration unless the user approves the updated test plan.

## Task And Evidence Migration

Do not reuse old task IDs for changed behavior. If work has not started, update task rows directly. If work is in progress or complete, add replacement or evolution rows and mark older rows as retained, superseded, replaced, not applicable, or skipped with approval.

Existing reports and worker evidence must remain auditable. Add new report rows or agent report fragments for changed behavior; do not rewrite old evidence to look like it was produced for the new spec.

If active worker branches or worktrees exist, decide per worker whether to continue, rebase, narrow scope, replace the task, or mark the branch superseded with explicit user approval. Never silently drop unintegrated worker changes.

## Resume Delivery

If evolution is documentation-only and behavior is unchanged, record why implementation and test tasks are not applicable, then run affected spec validation commands. Otherwise resume from the earliest affected execution phase: task breakdown, worktree provisioning, RED, GREEN, refactor/verify, or final review.

Final review for evolution must compare old and new acceptance IDs, RED/GREEN evidence, changed specs, migrated tasks, worker evidence, and residual risks.
