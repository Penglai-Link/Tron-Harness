# Acceptance: Init Rust Backend

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

Acceptance focuses on a small initialized backend that can be built, tested, and extended without ambiguity.

## Acceptance Criteria Matrix

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-BACKEND-01 | Backend Cargo package builds and exposes a reusable application router. | automated | `make be-check` and `make be-test` pass. | must |
| AC-BACKEND-02 | `GET /api/v1/health` returns the uniform `{ code, message, data }` response envelope. | automated | Router test asserts HTTP 200 and response fields. | must |
| AC-BACKEND-03 | Root Makefile provides backend install/check/test/run commands so project workflow starts with Makefile. | automated | `make help`, `make be-check`, and `make be-test` pass. | must |

## Business Acceptance

- [ ] AC-BACKEND-01 Backend initialization creates a maintainable Rust service foundation.
- [ ] AC-BACKEND-03 Root Makefile exposes backend workflow commands.

## UI Acceptance

- [ ] No UI changes are required for this feature.

## Backend Acceptance

- [ ] AC-BACKEND-01 Backend code is modular enough for future API/domain additions.
- [ ] AC-BACKEND-02 Health endpoint matches the uniform response body contract.
- [ ] AC-BACKEND-03 Makefile commands call Cargo backend commands from the repository root.

## Database Acceptance

- [ ] Database impact is marked not applicable.

## Integration Acceptance

- [ ] AC-BACKEND-01 Backend checks run from the root workflow.
- [ ] AC-BACKEND-02 Router-level tests verify API behavior without binding a port.

## Security And Privacy Acceptance

- [ ] Health endpoint returns no credentials, tenant data, user data, or private operational secrets.

## Regression Acceptance

- [ ] Existing frontend Makefile commands remain available.
- [ ] Rust backend initialization does not require frontend dependency installation.

## Evidence Requirements

- AC-BACKEND-01: `make be-check`, `make be-test`.
- AC-BACKEND-02: Rust router test for `GET /api/v1/health`.
- AC-BACKEND-03: `make help` shows backend targets and backend commands run through Makefile.
