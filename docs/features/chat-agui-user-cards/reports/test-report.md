# Test Report: Chat AG-UI User Cards

## Evidence Ledger

| ac_ref | command_ref | status | evidence |
| --- | --- | --- | --- |
| AC-AGUI-01 | CMD-FE-AGUI-BROWSER, CMD-FE-CHECK | passed | `rtk npm --prefix tron-frontend exec playwright -- test chat-agui-user-cards.spec.ts --reporter=line` passed. Browser selected Settings > AG-UI Debug > Off, sent a chat message, verified `AG-UI debug event cards` was absent, and the compact `Agent work rail` remained visible. `rtk make fe-check` passed with only the existing Vite chunk-size warning. |
| AC-AGUI-02 | CMD-FE-AGUI-BROWSER, CMD-FE-CHECK | passed | Browser proof found a compact work rail ahead of the assistant answer, verified one run rendered two collapsed `Thinking` cards for two reasoning segments, verified the first summary included `used 6 tools`, verified collapsed cards showed concise preview text for `Reviewing session context and available skills. Used 6 tools.` and `Summarizing tool results before replying.`, expanded the first card and saw only its long reasoning overflow content, then expanded the second card and verified it showed only the second segment text without leaking the first segment. |
| AC-AGUI-03 | CMD-FE-AGUI-BROWSER, CMD-FE-CHECK | passed | Browser proof found six compact tool steps nested inside the first `Thinking` card only, verified the nested group header `Used tools`, expanded `Run workspace.list`, verified parameters and `Listed workspaces` result, and verified the unmatched activity line did not appear. |
| AC-AGUI-04 | CMD-BE-TEST, CMD-BE-CHECK | passed | `rtk make be-check` and `rtk make be-test` passed. Focused backend tests prove runtime publishes `tool_call_id`, `tool_name`, args, result payloads, and protocol conversion emits matching `TOOL_CALL_START`, `TOOL_CALL_ARGS`, `ACTIVITY_DELTA`, `TOOL_CALL_RESULT`, and `TOOL_CALL_END` fields. |
| AC-AGUI-05 | CMD-FE-AGUI-BROWSER, CMD-BE-CHAT-STREAM | passed | Frontend regression injects stale prior-run events and verifies the visible `Worked for` duration and `Thinking` card only use the latest run. Backend regression `chat_stream_only_emits_events_for_current_run` verifies a second stream response does not replay the first run delta. |
| AC-AGUI-06 | CMD-BE-AGENT-RUNTIME, CMD-BE-CHAT-STREAM | passed | Runtime now publishes `agent.reasoning.delta` immediately after `agent.model.requested`, protocol conversion exposes it as `REASONING_MESSAGE_CONTENT`, and chat stream tests verify both internal reasoning progress and model reasoning deltas stream without entering final assistant output. |

## Final Validation

- `rtk make fe-check` passed.
- `rtk make be-check` passed.
- `rtk make be-test` passed.
- `rtk npm --prefix tron-frontend exec playwright -- test chat-agui-user-cards.spec.ts --reporter=line` passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test agent_runtime -- --nocapture` passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test chat_management_api chat_stream -- --nocapture` passed.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test agent_agui_stream_api -- --nocapture` passed.
- `rtk git diff --check` passed.
