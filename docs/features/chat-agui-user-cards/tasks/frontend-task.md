# Frontend Task: Chat AG-UI User Cards

## Task Rows

| ac_refs | phase | owner | files | status |
| --- | --- | --- | --- | --- |
| AC-AGUI-02, AC-AGUI-03 | tdd | lead-agent | `tron-frontend/src/domain/chat/pages/ChatManagementPage.tsx`, `tron-frontend/src/domain/chat/hooks/useChatMessageListBySession.ts`, `tron-frontend/tests/chat-agui-user-cards.spec.ts` | done |

## Notes

- Keep user-facing cards independent from debug-card visibility.
- Default grouped cards collapsed.
- Use `toolCallId` as the only match key for tool activity logs.
- Reasoning card aggregation must open a new card for every reasoning segment boundary inside the latest run, even when the stream reuses the same reasoning `messageId`.
- Tool steps should attach to the latest reasoning segment in the same run and render as compact nested child steps instead of separate top-level cards when a reasoning parent exists.
- User-facing work should read like a compact work rail ahead of the assistant answer, not like additional assistant bubbles.
