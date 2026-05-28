# Acceptance: Chat Management

## Acceptance Index

- AC-CM-01 [must] [database] Chat APIs persist sessions and messages as the durable history source, grouped by task with unassigned sessions separated; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CM-02 [must] [api] Chat send persists user and assistant messages around an agent run under the same session; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CM-03 [must] [interaction] `/chats` renders the three-column Chat Management workspace with collapsible grouped sessions, persisted message replay, composer send flow, and current-session highlight; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CM-04 [must] [interaction] The right context sidebar shows only app/task, worktree path/branch, actually observed AG-UI skills, and context window with unavailable fallback when backend statistics are absent; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CM-05 [must] [api] `POST /api/v1/chats/sessions/{sessionCode}/messages/stream` emits AG-UI standard SSE events instead of chat-internal `delta` and `done` events; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CM-06 [must] [interaction] The chat transcript renders AG-UI event cards for reasoning, tool calls, tool live logs, and run lifecycle events while streaming assistant Markdown incrementally; proof: @ref:spec/test-plan.md#proof-rows.
- AC-CM-07 [must] [interaction] Sending a chat message does not show a success bubble/toast after the stream completes; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
