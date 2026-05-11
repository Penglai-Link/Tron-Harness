# Test Scope: {{FEATURE_NAME}}

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: pending
- best_practice_defaults_applied: pending
- user_confirmation_status: waiting_human_decision

Summarize the lead agent's first-pass interpretation, recommended defaults, assumptions, and blocking confirmation questions. This section must exist before task planning.

## Scope Decision

- status: waiting_human_decision
- selected_profile: standard
- approved_by: none
- approved_at: none

Valid profiles: `minimal`, `standard`, `strict`, `custom`.

## Recommended Scope

Explain the recommended scope based on feature risk, database impact, API impact, UI impact, permissions, data sensitivity, and available repository test architecture.

## Included Test Layers

| layer | included | rationale | command or evidence |
| --- | --- | --- | --- |
| unit | yes | Core business behavior. | Not discovered. |
| integration | yes | Service/API/database behavior. | Not discovered. |
| api-contract | yes | Request/response compatibility. | Not discovered. |
| database-migration | if relevant | Schema and migration safety. | Not discovered. |
| frontend-component | if relevant | UI state and validation. | Not discovered. |
| browser-runtime | if relevant | Critical user journey evidence. | Not discovered. |
| performance | no | Add only when query or throughput risk exists. | Not applicable. |
| security | if relevant | Permissions, tenancy, and sensitive data. | Not discovered. |
| manual-review | yes | Traceability and final acceptance. | reports/review-report.md |

## Excluded Test Layers

List excluded layers and rationale. Avoid excluding a layer only because it is inconvenient when the feature risk requires it.

## Environment Requirements

Describe database, containers, browser, external services, mocks, credentials, generated clients, seed data, ports, and cleanup requirements.

## Test Boundary Matrix

Define the approved testing boundary before writing the executable test plan. This file decides which layers, risks, environments, and evidence categories are in scope; `spec/test-plan.md` later turns that scope into concrete cycles, files, fixtures, and commands.

| acceptance ids | required layers | risk or boundary | included | evidence expectation | command candidate | notes |
| --- | --- | --- | --- | --- | --- | --- |
| AC-MODULE-01 | unit/integration/api/frontend/browser | core business / boundary / error / permission / integration / regression | yes | Automated or approved manual evidence. | Not discovered. | Executable cycles belong in `spec/test-plan.md`. |

## Regression Command Candidates

| command | purpose | required for final | status |
| --- | --- | --- | --- |

## Evidence Required For Completion

Map required evidence to acceptance IDs, test layers, runtime evidence, review evidence, and any approved manual evidence. Cycle IDs are assigned later in `spec/test-plan.md`.

## Open Decisions

- Approve or modify the selected test scope before implementation.
