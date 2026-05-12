# 0-1 Feature Delivery Workflow

Use this reference for new features with no existing `docs/features/<feature-slug>/` workspace.

## Phase 1: Repository Discovery And Progressive Design Gates

Read `references/initial-design.md` before drafting.

Discover the current repository shape before drafting. Read product docs, route/module registries, API schemas, database migrations, ORM/entity/mapper files, package manifests, test configuration, CI configuration, and existing feature folders when present. Also scan sibling `*/spec/feature-spec.md` files under the feature root, normally `docs/features/`, to identify strong feature dependencies by feature spec content.

Keep this phase incremental. Each gate creates or updates only its listed file(s), runs the matching validation command, summarizes decisions, and stops for user confirmation unless the user explicitly authorized best-practice defaults. Do not create later spec files before their gate, final task/worktree/report files during Phase 1, or `spec/test-plan.md` before test scope approval.

| gate | start condition | create/update | draft focus | validation |
| --- | --- | --- | --- | --- |
| 1A Feature Spec | after discovery | `spec/feature-spec.md` | requirement, product boundary, goals/non-goals, roles, pages/entry points, DDD domain model, module boundaries, strong feature dependencies with dependency feature names, common capability extraction decisions, repository findings, assumptions, open decisions | `"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase spec --spec-stage feature` |
| 1B Database Design | `feature-spec.md` confirmed/corrected, or defaults approved | `spec/database-design.md` | impact decision, existing schema impact, proposed schema, relationships, migration, query/performance, governance, fixtures, validation commands; use `impact: no` or `impact: not_applicable` when persistence is unaffected | `"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase spec --spec-stage database` |
| 1C API Contract | `database-design.md` confirmed/corrected, or defaults approved | `spec/api-contract.md` | endpoints, request/response DTOs, parameter passing method per interface, auth, errors, frontend/backend boundary, database/transaction boundary, mocks/adapters | `"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase spec --spec-stage api` |
| 1D Acceptance And Test Scope | prior three spec files confirmed/corrected, or defaults approved | `spec/acceptance.md`, `spec/test-scope.md` | stable acceptance IDs, acceptance categories, evidence requirements, approved test boundary, included/excluded layers, risk coverage, environment, command candidates, and manual evidence decisions | `"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase spec --spec-stage acceptance` |

`spec/test-scope.md` must define `status: waiting_human_decision`, `status: approved`, or `status: approved_with_defaults`. Use `approved_with_defaults` only when the user has already said to proceed with the recommended test scope or best-practice defaults.

## Phase 2: Test Plan Gate And Complete Spec Validation

Apply user corrections to progressive design files. Create or update `spec/test-plan.md` only after `spec/test-scope.md` is approved or approved with defaults.

`spec/test-plan.md` is the executable test manual. In `TDD Cycle Table`, group cycles under `Positive Scenarios`, `Exception Scenarios`, and `Boundary Scenarios`. The TDD cycle ID is the only identifier in those tables and must use the same module segment as the primary acceptance ID, such as `AC-SNOWFLAKEID-01` -> `TDD-SNOWFLAKEID-01`. Keep TDD cycles fine-grained: one cycle proves one rule, branch, boundary, error, permission path, integration path, or regression risk. Use `TDD Cycle Details` only for important test matters not already present in the table: assertions, fixtures/data, boundary/error/permission cases, mocks/adapters, RED proof, and GREEN/VERIFY evidence.

When a test-plan section is outside the approved scope, write only `Not applicable.` for that section. Do not add explanatory paragraphs for excluded or irrelevant layers.

Set `Test Plan Decision` to `status: waiting_human_decision` unless the user has already reviewed this exact test plan and explicitly approved task breakdown.

During Phase 2, do not create or update `tasks/`, `orchestration/`, `reports/`, worktree plans, task IDs, owners, branches, or worker assignments. Phase 3 derives task rows from the approved `spec/test-plan.md`.

Run:

```bash
"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase spec --complete-spec --no-downstream-artifacts
```

Stop for user confirmation after complete spec validation. Do not begin task breakdown until `spec/test-plan.md` has `Test Plan Decision` status `approved` or `approved_with_defaults`.

## Phase 3: Multi-Agent Task Breakdown

Start only after `spec/test-plan.md` has `Test Plan Decision` status `approved` or `approved_with_defaults`. Create or update all task files in `tasks/`. Each task row must include:

```text
id | phase | owner | depends on | scope | acceptance ids | status
```

Keep `xxx-task.md` rows as a lightweight execution index. Do not put worker branch names, worktree paths, validation commands, agent roles, or report targets in the task table. Those belong in `orchestration/worktree-plan.md`, `spec/test-plan.md`, and the matching `reports/*` files.

Task sequencing must enforce TDD:

- RED rows live in `tasks/test-task.md`
- GREEN rows live in `tasks/database-task.md`, `tasks/backend-task.md`, or `tasks/frontend-task.md`
- GREEN rows depend on matching RED rows
- REFACTOR rows depend on GREEN rows
- VERIFY and REVIEW rows run after reports or integration evidence exists
- every non-lead task row has a scope clear enough for `orchestration/worktree-plan.md` to assign a dedicated worker branch and worktree

Run:

```bash
"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase tasks
```

Set task status to `waiting_human_decision` and stop before implementation unless the user already approved execution.

## Phase 4: Worktree Provisioning

Read `references/multi-agent-worktree-orchestration.md`, `references/git-worktree-orchestration.md`, and the selected `eliteforge-git-feature-oriented-*` skill before delegating.

The lead agent creates or verifies the selected-git-skill multi-agent base branch and per-worker worktrees. Do not use feature-skill-defined branch or path names. Resolve the main branch, main worktree path, subagent branch, subagent worktree path, `agentId`, and subtask names from the selected git skill, then create worktrees:

```bash
git status --short
git switch -c "<selected-git-skill-multi-agent-base-branch>"  # only when no base branch exists
"$skill_dir/scripts/create_agent_worktree.sh" "<selected-git-skill-subagent-branch>" "<selected-git-skill-subagent-worktree-path>" "<selected-git-skill-multi-agent-base-branch>"
```

Record the selected git skill, main branch, main worktree path, every subagent worktree, every subagent branch, base branch, task ID, and status in `reports/worktree-report.md` and `orchestration/worktree-plan.md`.

Run:

```bash
"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase worktrees
```

## Phase 5: RED Multi-Agent Execution

Assign RED rows first. RED workers may edit only tests, fixtures, test utilities, and explicitly assigned generated test assets. They must not implement production behavior.

A valid RED report must include task IDs, acceptance IDs, changed test paths, command run, expected failure summary, why the failure proves missing behavior rather than environment breakage, and worktree path.

The lead agent verifies RED evidence and merges or replays RED changes into the selected-git-skill multi-agent base branch before assigning dependent GREEN rows.

## Phase 6: GREEN Multi-Agent Implementation

Assign GREEN rows only after matching RED evidence is present in the selected-git-skill multi-agent base branch or an explicit legacy-coverage exception is recorded.

GREEN workers implement the smallest production change needed to pass. They must not weaken tests. They update their assigned report and return changed paths, commands, acceptance mapping, blockers, and worktree path.

The lead agent merges GREEN branches into the selected-git-skill multi-agent base branch, resolves conflicts only within lead-owned integration scope, and reruns affected validation.

## Phase 7: REFACTOR, Regression, And Review

Assign REFACTOR rows only after GREEN is verified. Refactor workers must preserve behavior and rerun affected commands.

The lead agent creates or updates all report files under `reports/`. Final review compares implementation evidence against `spec/acceptance.md`, `spec/test-scope.md`, `spec/test-plan.md`, `spec/database-design.md`, and `reports/worktree-report.md`.

Run:

```bash
"$skill_dir/scripts/validate_feature_docs.py" <feature-dir> --phase final
```

Do not claim completion while final validation fails, reports contain unresolved blockers, required RED/GREEN/refactor evidence is missing, worktree merge evidence is missing, or required runtime evidence is missing.
