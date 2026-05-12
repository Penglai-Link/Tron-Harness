# Test Scope: Init Rust Backend

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The selected scope is intentionally small and automated: Rust build checks, unit/config tests, and router-level API tests. Browser and database layers are outside this initialization slice.

## Scope Decision

- status: approved_with_defaults
- selected_profile: standard
- approved_by: user execution request and lead-agent defaults
- approved_at: 2026-05-12

Valid profiles: `minimal`, `standard`, `strict`, `custom`.

## Recommended Scope

Use a standard backend bootstrap scope. The feature introduces backend runtime and API shape but no database schema, user permissions, or frontend behavior. The critical risks are wrong response envelope, non-runnable Cargo package, and missing Makefile workflow.

## Included Test Layers

| layer | included | rationale | command or evidence |
| --- | --- | --- | --- |
| unit | yes | Config defaults and response DTOs should be deterministic. | `make be-test` |
| integration | yes | Router-level health endpoint behavior must be verified. | `make be-test` |
| api-contract | yes | Uniform response shape is a project-level API rule. | `make be-test` |
| database-migration | no | No persistence impact. | Not applicable. |
| frontend-component | no | No frontend changes. | Not applicable. |
| browser-runtime | no | No user-facing UI path is introduced. | Not applicable. |
| performance | no | Health endpoint is constant-time and low risk. | Not applicable. |
| security | yes | Health endpoint must not expose sensitive data. | `make be-test` |
| manual-review | yes | Final review maps commands to acceptance criteria. | `reports/review-report.md` |

## Excluded Test Layers

- Database migration tests are excluded because this feature creates no schema.
- Frontend component and browser runtime tests are excluded because this feature changes no UI.
- Performance tests are excluded because no query, queue, or high-throughput path is introduced.

## Environment Requirements

- Rust toolchain with Cargo.
- Network access only for initial dependency resolution.
- No database, container, browser, external service, credential, or frontend build is required.

## Test Boundary Matrix

| acceptance ids | required layers | risk or boundary | included | evidence expectation | command candidate | notes |
| --- | --- | --- | --- | --- | --- | --- |
| AC-BACKEND-01 | unit/integration | backend package and router initialization | yes | Cargo check and tests pass. | `make be-check`, `make be-test` | No port binding needed for tests. |
| AC-BACKEND-02 | api-contract/integration/security | uniform health response and safe metadata | yes | Router test asserts response body. | `make be-test` | Must use `{ code, message, data }`. |
| AC-BACKEND-03 | integration/manual-review | root Makefile backend workflow | yes | Make targets are listed and runnable. | `make help`, `make be-check`, `make be-test` | Existing frontend targets remain. |

## Regression Command Candidates

| command | purpose | required for final | status |
| --- | --- | --- | --- |
| `make help` | show frontend and backend workflow targets | yes | ready |
| `make be-check` | compile backend package | yes | ready |
| `make be-test` | run backend tests | yes | ready |

## Evidence Required For Completion

Final evidence must include the command output status for `make help`, `make be-check`, and `make be-test`, plus a review note that no database or frontend runtime evidence is required for this slice.

## Open Decisions

- None.
