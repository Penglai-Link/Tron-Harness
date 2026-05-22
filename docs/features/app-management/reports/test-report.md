# Test Report: App Management

## Summary

Focused verification passed across backend, frontend compile and build, and real Playwright flows. No open test blocker remains for the implemented scope.

## Evidence Rows

- AC-APP-01 | red: placeholder list UI and import actions would have failed browser expectations | green: `tests/app-management-smoke.spec.ts` passed list, filter, and import-removal coverage | command_ref: CMD-PW-APP | browser: clicks=list filters and pagination controls; inputs=search text; network=workspace app list queries; feedback=correct rows and no import action; artifacts=Playwright report | status: done.
- AC-APP-02 | red: missing create page and route | green: `tests/app-management-smoke.spec.ts` create flow passed | command_ref: CMD-PW-APP | browser: clicks=create and save actions; inputs=app form values; network=create request; feedback=detail route landed; artifacts=Playwright report | status: done.
- AC-APP-03 | red: missing detail and edit routes | green: `tests/app-management-smoke.spec.ts` edit flow passed | command_ref: CMD-PW-APP | browser: clicks=detail and edit save actions; inputs=updated metadata; network=patch request; feedback=detail view updated; artifacts=Playwright report | status: done.
- AC-APP-04 | red: no lifecycle browser flow existed | green: `tests/app-management-smoke.spec.ts` archive and restore coverage passed while backend lifecycle tests stayed green | command_ref: CMD-PW-APP | browser: clicks=archive and restore actions; inputs=archived filter; network=lifecycle requests; feedback=status transitions visible; artifacts=Playwright report | status: done.
- AC-APP-05 | red: no app API contract coverage existed | green: `make be-test` passed app API integration coverage | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-APP-06 | red: task app selection depended on free-text input and legacy codes always resent | green: task smoke passed and backend invalid-code tests stayed green | command_ref: CMD-PW-TASK | browser: clicks=app selection and legacy removal actions; inputs=task fields; network=task create and patch bodies omit or include `appCodes` correctly; feedback=toasts and visible legacy chips; artifacts=Playwright report | status: done.

## Changed Paths

- `tron-backend/tests/app_management_api.rs`
- `tron-backend/tests/task_management_api.rs`
- `tron-frontend/tests/app-management-smoke.spec.ts`
- `tron-frontend/tests/task-management-smoke.spec.ts`

## Commands

- `make fe-check`
- `make be-test`
- `cd tron-frontend && npx playwright test tests/app-management-smoke.spec.ts tests/task-management-smoke.spec.ts`

## Blockers

- None.
