# Frontend Task: Chat AG-UI User Cards

## Task Rows

| ac_refs | phase | owner | files | status |
| --- | --- | --- | --- | --- |
| AC-AGUI-01, AC-AGUI-02, AC-AGUI-03 | tdd | lead-agent | `tron-frontend/src/domain/chat/pages/ChatManagementPage.tsx`, `tron-frontend/src/domain/settings/model/aguiDebugPreferences.ts`, `tron-frontend/src/domain/settings/pages/SettingsGeneralPage.tsx`, `tron-frontend/src/domain/chat/hooks/useChatMessageListBySession.ts`, `tron-frontend/tests/chat-agui-user-cards.spec.ts` | done |

## Notes

- Keep user-facing cards independent from debug-card visibility.
- Default grouped cards collapsed.
- Use `toolCallId` as the only match key for tool activity logs.
