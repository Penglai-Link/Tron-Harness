# Test Report: Chat Management

## Summary

All AC rows are implemented and verified. API-mode send was additionally verified against the runtime default model on 2026-05-27. The 2026-05-28 continuation changed chat streaming to AG-UI standard events through the EventBus adapter path. The 2026-05-30 continuation added chat agent selection and workspace-resolved system CLI runtime dispatch (`codex`/`opencode`).

## Evidence Rows

- AC-CM-01 | red: `rtk make be-test` failed before implementation because `/api/v1/chats/*` routes and tables were absent | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api -- --nocapture` passed session creation, task grouping, Unassigned grouping, and empty persisted message replay | command_ref: CMD-BE-TEST | browser: clicks=not_applicable; inputs=not_applicable; network=POST /api/v1/chats/sessions,GET /api/v1/chats/session-groups,GET /api/v1/chats/sessions/{sessionCode}/messages; feedback=uniform SUCCESS envelopes with persisted groups and messages; artifacts=tron-backend/tests/chat_management_api.rs | status: done.
- AC-CM-02 | red: the same RED run proved chat send had no API boundary and no assistant message persistence | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api -- --nocapture` passed send orchestration, persisted user and assistant messages, returned run code, and `agent_runs.conversation_code=sessionCode` assertion | command_ref: CMD-BE-TEST | browser: clicks=not_applicable; inputs=chat message content and mock model config; network=POST /api/v1/chats/sessions/{sessionCode}/messages,GET /api/v1/agents/runs?conversationCode={sessionCode}; feedback=uniform SUCCESS envelope returns session run summary and two persisted messages; artifacts=tron-backend/tests/chat_management_api.rs | status: done.
- AC-CM-02 | verify: runtime API-mode send against the configured default `RESPONSES` model completed after fixing restarted run-code collisions and Responses reasoning/message parsing | command_ref: CMD-BE-TEST | browser: clicks=not_applicable; inputs=`Say one short sentence proving the chat runtime is working. Include the word TRON.`; network=POST /api/v1/auth/login,POST /api/v1/chats/sessions,POST /api/v1/chats/sessions/SESSION-735200000000000011/messages,GET /api/v1/chats/sessions/SESSION-735200000000000011/messages; feedback=run `RUN-1779898334536-9B7F105E44B0060E` returned `COMPLETED`, persisted roles were `USER,ASSISTANT`, assistant text was visible from persisted API data; artifacts=reports/chat-management-real-response-network.json | status: done.
- AC-CM-02 | verify: runtime API-mode stream send completed through `/api/v1/chats/sessions/{sessionCode}/messages/stream` after adding provider-side streaming callbacks and Responses delta-only parsing | command_ref: CMD-BE-TEST,CMD-FE-CHECK | browser: clicks=Sign In,Send; inputs=`Say one short sentence proving streaming chat works. Include STREAM-1779900217323.`; network=POST /api/v1/auth/login,GET /api/v1/models/workspaces/WS-C87ED739C5EEC54C,GET /api/v1/chats/session-groups,GET /api/v1/chats/sessions/SESSION-735200000000000011/messages,POST /api/v1/chats/sessions/SESSION-735200000000000011/messages/stream; feedback=stream returned `text/event-stream`, 6 delta events, done event with run `RUN-1779900220935-F7EFDC1FF9BB3B95`, assistant text contained the STREAM token once, and the page did not white-screen; artifacts=reports/chat-management-stream-response.png,reports/chat-management-stream-response-network.json | status: done.
- AC-CM-03 | red: existing `/chats` redirected to a static sample session with no collapsible session management | green: `rtk make fe-check` passed typecheck and build after replacing the static pages with the Chat Management workbench | command_ref: CMD-FE-CHECK | browser: clicks=sign-in,collapse-session-sidebar,expand-session-sidebar,send-message; inputs=auth credentials,chat composer text; network=auth endpoints mocked, chat data from mock service mode; feedback=SESSION-8842 transcript receives the sent message and assistant response; artifacts=reports/chat-management-smoke.png | status: done.
- AC-CM-03 | verify: API-mode frontend loaded the persisted runtime session and rendered the real assistant response | command_ref: CMD-FE-CHECK | browser: clicks=sign-in; inputs=runtime admin credentials; network=GET /api/v1/chats/session-groups,GET /api/v1/chats/sessions/SESSION-735200000000000011/messages; feedback=`SESSION-735200000000000011` and `This response proves the chat runtime is working and ready to enter the TRON grid.` were visible on `/chats`; artifacts=reports/chat-management-real-response.png,reports/chat-management-real-response-network.json | status: done.
- AC-CM-04 | red: the existing right panel showed generic Utility Hub data and fabricated model/token/tool values | green: `rtk make fe-check` passed and browser smoke verified the shell right sidebar shows Run Context with app/task, worktree path/branch, used skill, and unavailable context-window fallback | command_ref: CMD-FE-CHECK | browser: clicks=open-right-sidebar; inputs=none; network=auth endpoints mocked, AG-UI events from mock service mode; feedback=eliteforge-codex-superpower visible as an actually observed skill and context window shows unavailable; artifacts=reports/chat-management-smoke.png | status: done.
- AC-CM-05 | red: chat stream previously emitted chat-internal `delta` and `done` SSE events | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_stream_persists_messages_and_emits_deltas -- --nocapture` passed with `RUN_STARTED`, `TEXT_MESSAGE_START`, `TEXT_MESSAGE_CONTENT`, `TEXT_MESSAGE_END`, `RUN_FINISHED`, and no `event: delta` or `event: done`; implementation follows ReactAgent internal event -> EventBus -> AG-UI stream sink -> protocol adapter -> chat SSE | command_ref: CMD-BE-TEST | browser: clicks=not_applicable; inputs=`stream this session`; network=POST /api/v1/chats/sessions/{sessionCode}/messages/stream; feedback=text/event-stream contains AG-UI standard events and persisted messages remain correct; artifacts=tron-backend/tests/chat_management_api.rs | status: done.
- AC-CM-06 | red: transcript only rendered plain chat bubbles and raw delta appends | green: `rtk make fe-check` passed after adding `ChatAguiStreamEvent` parsing, AG-UI event cards for reasoning/tool/activity/run events, mock live events, and `MarkdownContent` rendering for assistant messages while deltas stream | command_ref: CMD-FE-CHECK | browser: clicks=attempted; inputs=mock/API AG-UI SSE frames; network=stream parser covers `RUN_STARTED`, `REASONING_MESSAGE_CONTENT`, `TOOL_CALL_START`, `ACTIVITY_DELTA`, `TEXT_MESSAGE_CONTENT`, and `CUSTOM chat.result`; feedback=typed build verifies Thinking, Tool call, Tool log card branches and assistant Markdown region; artifacts=tron-frontend/src/domain/chat/pages/ChatManagementPage.tsx | status: done.
- AC-CM-07 | red: completed send path showed `Message sent` success toast | green: `rtk make fe-check` passed after removing send-success toast while preserving error/validation toasts and transcript/session updates | command_ref: CMD-FE-CHECK | browser: clicks=attempted; inputs=send completion path; network=standard `CUSTOM chat.result` AG-UI event resolves final `ChatSendResult`; feedback=no `toast.success("Message sent")` call remains in chat send completion path; artifacts=tron-frontend/src/domain/chat/pages/ChatManagementPage.tsx | status: done.
- AC-CM-08 | red: composer footer supported model only and new-session payload could not persist `agentCode` | green: `rtk make fe-check` passed after adding `ChatAgentSelector`, workspace agent loading, active-session agent rebinding, and create/send payload wiring for `agentCode` | command_ref: CMD-FE-CHECK | browser: clicks=open-agent-selector,select-agent,create-session; inputs=select `codex` then create chat; network=GET /api/v1/agents/workspaces/{workspaceCode}/system,POST /api/v1/chats/sessions; feedback=UI shows selected agent badge and session payload includes selected `agentCode`; artifacts=tron-frontend/src/domain/chat/pages/ChatManagementPage.tsx,tron-frontend/src/domain/chat/model/ChatModel.ts | status: done.
- AC-CM-09 | red: send path always used in-process React runtime and had no system agent CLI adapter routing | green: `rtk make be-test` passed with `chat_send_uses_codex_system_agent_cli_runtime` and `chat_send_uses_opencode_system_agent_cli_runtime`, asserting real command execution, output parsing, and token usage mapping; session creation persistence is covered by `chat_session_creation_persists_selected_agent_code` | command_ref: CMD-BE-TEST | browser: clicks=not_applicable; inputs=`agentCode=codex|opencode` with chat send payload; network=POST /api/v1/chats/sessions,POST /api/v1/chats/sessions/{sessionCode}/messages; feedback=assistant message persisted from CLI output and run token usage reflects CLI JSON records; artifacts=tron-backend/tests/chat_management_api.rs,tron-backend/src/application/agent/system_cli_model_client.rs | status: done.

## Changed Paths

- `tron-backend/src/domain/chat*`
- `tron-backend/src/application/agent/invocation.rs`
- `tron-backend/src/application/agent/management.rs`
- `tron-backend/src/application/agent/system_cli_model_client.rs`
- `tron-backend/src/application/agent/agui_stream.rs`
- `tron-backend/src/application/agent/runtime.rs`
- `tron-backend/src/domain/agent/agui.rs`
- `tron-backend/src/domain/agent/kind.rs`
- `tron-backend/src/domain/agent/model_exchange.rs`
- `tron-backend/src/application/chat*`
- `tron-backend/src/adapters/inbound/http/chats*`
- `tron-backend/src/adapters/outbound/model/protocol_adapters.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/chat_repository.rs`
- `tron-backend/migrations/20260527_0018_create_chat_management.sql`
- `tron-backend/migrations/20260530_0023_add_chat_session_agent_code.sql`
- `tron-backend/tests/chat_management_api.rs`
- `tron-frontend/src/domain/chat/**`
- `tron-frontend/src/domain/chat/model/ChatModel.ts`
- `tron-frontend/src/domain/chat/mock/data.ts`
- `tron-frontend/src/router/routes.tsx`
- `tron-frontend/src/router/pageRegistry.ts`
- `tron-frontend/src/app/App.tsx`

## Commands

- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api -- --nocapture`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_session_creation_persists_selected_agent_code -- --nocapture`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_send_uses_codex_system_agent_cli_runtime -- --nocapture`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_send_uses_opencode_system_agent_cli_runtime -- --nocapture`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_send_does_not_reuse_existing_run_codes_after_restart -- --nocapture`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_stream_persists_messages_and_emits_deltas -- --nocapture`: passed.
- `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/chat-management --phase spec --complete-spec`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml responses_response_skips_reasoning_and_reads_message_output_text -- --nocapture`: passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml responses_stream_maps_reasoning_and_output_deltas -- --nocapture`: passed.
- `rtk make be-test`: passed; existing test warnings remain.
- `rtk make fe-check`: passed; Vite reported existing chunk-size and Node deprecation warnings.
- Browser smoke with Playwright against `http://127.0.0.1:5173/chats`: passed; artifact `reports/chat-management-smoke.png`.
- API-mode frontend verification with Playwright against `http://localhost:5173/chats`: passed; artifacts `reports/chat-management-real-response.png` and `reports/chat-management-real-response-network.json`.
- API-mode AG-UI card verification with Playwright against `http://127.0.0.1:5173/chats`: passed with mocked authenticated API responses and standard AG-UI stream frames; artifact `reports/chat-management-agui-cards.png`.

## Blockers

- None.
