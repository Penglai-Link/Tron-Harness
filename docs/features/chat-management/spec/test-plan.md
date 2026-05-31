# Test Plan: Chat Management

## Test Plan Decision

- status: waiting_human_decision
- approved_by: pending
- approved_at: pending

## Proof Rows

- AC-CM-01 | layer: api | case: session and message persistence | red: chat routes and tables are absent | green: backend test creates and lists sessions and messages through `/api/v1/chats` with task grouping and no AG-UI-derived history | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-CM-02 | layer: api | case: send orchestration | red: frontend would have to call `/api/v1/agents/runs` directly and no assistant message is persisted | green: backend test posts to chat send endpoint, verifies persisted user and assistant messages, returned run code, and run history conversation code | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-CM-03 | layer: frontend | case: chat management workbench | red: `/chats` redirects to a static sample session and has no collapsible session management | green: frontend build verifies the new workbench, grouped session rail, composer flow, and current-session state compile cleanly | interaction: visit `/chats` -> collapse and expand session sidebar -> select grouped sessions -> send a mock message -> verify visible persisted-style transcript update | command_ref: CMD-FE-CHECK.
- AC-CM-04 | layer: frontend | case: right context contract | red: right panel shows generic utility tabs and fabricated model/token/tool data | green: frontend build verifies the right context sidebar renders only the four allowed context groups and displays `unavailable` for missing context-window data | interaction: visit `/chats` -> inspect right sidebar headings and skill list after mock AG-UI events | command_ref: CMD-FE-CHECK.
- AC-CM-05 | layer: api | case: AG-UI stream contract | red: stream sends chat-internal `delta` and `done` events | green: backend stream test asserts `RUN_STARTED`, `TEXT_MESSAGE_START`, `TEXT_MESSAGE_CONTENT`, `TEXT_MESSAGE_END`, and `RUN_FINISHED` events and rejects internal event names | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-CM-06 | layer: frontend | case: AG-UI event cards and Markdown | red: transcript only renders plain chat bubbles and appends raw deltas | green: frontend build verifies AG-UI card rendering branches and assistant Markdown renderer compile against live stream events | interaction: send a mock message -> inspect reasoning/tool/log card labels and incrementally rendered assistant Markdown | command_ref: CMD-FE-CHECK.
- AC-CM-07 | layer: frontend | case: send completion feedback | red: completed send shows `Message sent` success toast | green: frontend build verifies the send completion path updates transcript/session state without success toast | interaction: send a mock message -> verify transcript updates and no success toast is created | command_ref: CMD-FE-CHECK.
- AC-CM-08 | layer: interaction | case: selectable chat agent and session persistence | red: composer footer supports model only and session create cannot persist `agentCode` | green: frontend build verifies agent selector renders `main-agent` plus workspace-enabled system agents, create-session request includes selected `agentCode`, and active session restores agent selection | interaction: visit `/chats` -> open agent selector -> choose `codex` -> create session -> verify header/session payload reflect `agentCode=codex` | command_ref: CMD-FE-CHECK.
- AC-CM-09 | layer: api | case: chat integration with runtime dispatch | red: send path always runs in-process React runtime and ignores workspace system agent policy/command resolution | green: backend tests assert `agentCode=codex` and `agentCode=opencode` consume runtime dispatch capability, parse runtime output/token usage, and persist assistant response under chat message history | interaction: not applicable | command_ref: CMD-BE-TEST.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red/green/verify | rerun: after backend API, persistence, or agent orchestration changes | expect: backend tests pass including chat management API tests.
- CMD-FE-CHECK | command: `rtk make fe-check` | cost: medium | when: green/verify | rerun: after frontend chat changes | expect: frontend typecheck and build pass.
- CMD-DOCS-SPEC | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/chat-management --phase spec --complete-spec` | cost: fast | when: docs | rerun: after feature spec changes | expect: feature docs spec gate passes.

## Evidence Rules

- Every `AC-*` from `spec/acceptance.md` must have a proof row.
- Browser proof must include concrete clicks, visible feedback, network assertions, and artifacts if claimed.
- Final reports record RED, GREEN, VERIFY, command output, and any browser evidence.
