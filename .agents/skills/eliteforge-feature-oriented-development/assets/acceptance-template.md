# Acceptance: {{FEATURE_NAME}}

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: pending
- best_practice_defaults_applied: pending
- user_confirmation_status: waiting_human_decision

Summarize the lead agent's first-pass interpretation, recommended defaults, assumptions, and blocking confirmation questions. This section must exist before task planning.

## Acceptance Criteria Matrix

Use `AC-<MODULE>-NN` format. `<MODULE>` is the current module or bounded-context token and must match the module segment used by related TDD cycle IDs.

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-MODULE-01 | Business outcome is satisfied. | automated | Link to TDD cycle and command. | must |

## Business Acceptance

- [ ] AC-MODULE-01 Business outcome is satisfied.

## UI Acceptance

- [ ] Required pages, states, and interactions are implemented and verified when relevant.

## Backend Acceptance

- [ ] Backend behavior, validation, permissions, and errors match the API contract.

## Database Acceptance

- [ ] Database impact is explicitly approved or marked not applicable.
- [ ] Schema, migration, index, fixture, repository, and compatibility evidence exists when relevant.

## Integration Acceptance

- [ ] Frontend/backend/database integration paths are verified with realistic data when relevant.

## Security And Privacy Acceptance

- [ ] Permissions, tenancy, audit, privacy, masking, and sensitive-data behavior are verified when relevant.

## Regression Acceptance

- [ ] Existing critical behavior remains covered by automated or approved manual evidence.

## Evidence Requirements

List required commands, browser scenarios, screenshots, logs, test reports, worktree evidence, or review artifacts. Each evidence item must map to one or more acceptance IDs and TDD cycle IDs.
