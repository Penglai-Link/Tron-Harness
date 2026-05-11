# Subagent Orchestration Contract

Use this reference when task files exist and the user has approved execution.

## Main Agent

The Main Agent is the first agent that activated the skill. It owns user communication, feature documents, confirmation gates, TDD ordering, worktree setup, dispatch, integration, final validation, and completion claims.

Main Agent responsibilities:

- read all feature docs before assigning work
- group ready task rows by dependency, TDD phase, acceptance IDs, and write scope
- create one dedicated git worktree and branch for each subagent using `eliteforge/<agent-id>/<feature-slug>` order
- run RED tasks before dependent GREEN tasks
- avoid parallel workers that may edit the same files or tightly coupled modules
- keep DTOs, API boundaries, migrations, fixtures, route contracts, and acceptance evidence consistent
- merge worker report fragments into canonical reports
- integrate worker diffs only after scope, evidence, and validation checks

## Explorer Subagents

Use explorer subagents for read-only repository questions that unblock more than one task. They still use a dedicated worktree.

Prompt shape:

```text
You are an explorer subagent for feature <feature-slug>.
Use only this worktree: <worktree-path>. The path must use `eliteforge/<agent-id>/<feature-slug>` order.
Branch: <branch>.
Read <specific files or directories>. Answer only these questions: <questions>.
Do not edit files.
Return facts, paths, line references when useful, database/test conventions, commands, and risks.
```

## RED Test Subagents

Use RED test subagents for RED task rows in `tasks/test-task.md`.

Prompt shape:

```text
You are a RED test subagent for feature <feature-slug>.
Use only this worktree: <worktree-path>. The path must use `eliteforge/<agent-id>/<feature-slug>` order.
Branch: <branch>.
Implement RED task rows <ids>.

Read:
- docs/features/<feature-slug>/spec/feature-spec.md
- docs/features/<feature-slug>/spec/database-design.md
- docs/features/<feature-slug>/spec/api-contract.md
- docs/features/<feature-slug>/spec/acceptance.md
- docs/features/<feature-slug>/spec/test-scope.md
- docs/features/<feature-slug>/spec/test-plan.md
- docs/features/<feature-slug>/tasks/test-task.md
- references/tdd-workflow.md

Write scope:
- <paths/modules>

Add or update tests only. Do not implement production behavior except explicit fixture or test harness setup. Run the validation command and confirm the failure is expected. Write a report fragment under reports/agents. Final answer must list changed files, commands, RED failure reason, acceptance IDs, TDD cycle IDs, and blockers.
```

## Database Subagents

Use database subagents for migration, schema, repository, fixture, data-access, rollback, and compatibility tasks.

Prompt shape:

```text
You are a database subagent for feature <feature-slug>.
Use only this worktree: <worktree-path>. The path must use `eliteforge/<agent-id>/<feature-slug>` order.
Branch: <branch>.
Implement task rows <ids>.
Read the feature docs and current migration/data-access conventions.
Stay within these write scopes: <paths/modules>.
Respect the approved database design. Do not alter schema shape, migration order, tenancy, indexes, rollback, or compatibility strategy beyond the accepted design. Run assigned migration/schema/repository tests and write a report fragment.
```

## GREEN Implementation Subagents

Use GREEN workers for implementation tasks with clear write ownership and integrated RED evidence.

Prompt shape:

```text
You are a GREEN implementation subagent for feature <feature-slug>.
Use only this worktree: <worktree-path>. The path must use `eliteforge/<agent-id>/<feature-slug>` order.
Branch: <branch>.
Implement GREEN task rows <ids>.

Read:
- all files under docs/features/<feature-slug>/spec/
- relevant task file
- reports/test-report.md or the matching RED report fragment

Write scope:
- <paths/modules>

Make the smallest production change that turns the relevant RED tests GREEN. Do not weaken tests. Do not edit outside your scope unless you record why. Write a report fragment. Final answer must list changed files, validation results, acceptance IDs, TDD cycle IDs, and blockers.
```

## Refactor And Verify Subagents

Use refactor subagents only after GREEN evidence exists. Use verify subagents after integration. They must run the approved validation commands and record evidence.

## Review Subagents

Use review subagents after reports exist. Review is read-mostly unless the task row explicitly allows small fixes. Review output should cover database alignment, TDD evidence, test-scope compliance, worktree integration, changed paths, acceptance evidence, and final status recommendation.

## Dynamic Batching Rules

1. Parse rows from `database-task.md`, `backend-task.md`, `frontend-task.md`, `test-task.md`, and `review-task.md`.
2. Exclude rows with unresolved dependencies or non-ready status.
3. Execute for the same acceptance IDs in order: explore/support → RED → GREEN → refactor/verify → review.
4. Do not batch RED with dependent GREEN.
5. Batch rows only when write scopes do not overlap.
6. Keep canonical docs and report merging in the Main Agent.
7. Wait for worker results before assigning dependent rows.
