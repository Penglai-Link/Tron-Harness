# Backend Task: Variable Sensitive Data

## Execution Contract

- feature_slug: variable-sensitive-data
- status: done
- execution_mode: hybrid

## TDD Contract

One backend worker should own RED -> GREEN -> VERIFY for storage, display, edit, reveal, runtime, and non-sensitive regression because the affected service and DTO files overlap heavily.

## Task Rows

- AC-VSD-03 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/application/variable; tron-backend/src/adapters/inbound/http/variables; tron-backend/src/adapters/inbound/http/admin/variables; tron-backend/tests/variable_management_api.rs | command_ref: CMD-BE-VARIABLE-TEST | status: done.
- AC-VSD-04 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/application/variable; tron-backend/src/adapters/inbound/http/variables; tron-backend/src/adapters/inbound/http/admin/variables; tron-backend/tests/variable_management_api.rs | command_ref: CMD-BE-VARIABLE-TEST | status: done.
- AC-VSD-05 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/application/user; tron-backend/src/adapters/inbound/http/auth; tron-backend/src/application/variable; tron-backend/src/adapters/inbound/http/variables; tron-backend/tests/auth_bearer_context.rs; tron-backend/tests/variable_management_api.rs | command_ref: CMD-BE-VARIABLE-TEST | status: done.
- AC-VSD-06 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/application/agent/invocation.rs; tron-backend/src/application/agent/runtime_adapter; tron-backend/src/application/variable; tron-backend/tests/agent_runtime.rs; tron-backend/tests/variable_management_api.rs | command_ref: CMD-BE-VARIABLE-TEST | status: done.
- AC-VSD-10 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/src/application/variable; tron-backend/tests/variable_management_api.rs; tron-backend/tests/agent_runtime.rs | command_ref: CMD-BE-VARIABLE-TEST | status: done.

## Worktree Notes

- Planned worker: @ref:orchestration/worktree-plan.md#wt-01.
- Backend worker owns API DTO names consumed by the frontend worker.
- Frontend worker should not start implementation until these contracts are stable or explicitly handed off.

## Integration Notes

- Step-up must verify the authenticated user without refreshing login sessions.
- Reveal handlers must get user identity from request auth context, not request body.
- Runtime resolver must not pass display masks to `AgentRunContext`.

## Open Decisions

- None.
