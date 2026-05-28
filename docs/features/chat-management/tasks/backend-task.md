# Backend Task: Chat Management

## Execution Contract

- feature_slug: chat-management
- status: done
- execution_mode: solo

## TDD Contract

Backend rows cover RED -> GREEN -> focused VERIFY in the selected Lite Spec base worktree.

## Task Rows

- AC-CM-01 | phase: tdd | owner: lead | scope: `tron-backend/src/domain/chat*`, `tron-backend/src/application/chat*`, `tron-backend/src/adapters/outbound/persistence/sqlite/chat_repository.rs`, `tron-backend/src/adapters/inbound/http/chats*`, `tron-backend/migrations/20260527_0018_create_chat_management.sql`, `tron-backend/tests/chat_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-CM-02 | phase: tdd | owner: lead | scope: `tron-backend/src/application/chat/service.rs`, existing agent invocation wiring, `tron-backend/tests/chat_management_api.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-CM-05 | phase: tdd | owner: lead | scope: `tron-backend/src/application/agent/runtime.rs`, `tron-backend/src/application/agent/agui_stream.rs`, `tron-backend/src/domain/agent/model_exchange.rs`, `tron-backend/src/adapters/inbound/http/chats/handler.rs`, `tron-backend/tests/chat_management_api.rs` | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

Rows ran in the selected Lite Spec base worktree because backend chat service, persistence, routes, and orchestration were tightly coupled.

## Integration Notes

Chat send uses existing `AgentInvocationService` and preserves `/api/v1/agents/runs` as a direct execution API outside the Chat UI boundary.

Chat stream now keeps the event path as ReactAgent internal events -> EventBus -> AG-UI stream sink -> AG-UI protocol adapter -> chat SSE response. The chat handler no longer sends chat-internal `delta`/`done` events.

## Open Decisions

- None.
