# Test Plan: Chat AG-UI User Cards

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-28

## Proof Rows

- AC-AGUI-01 | layer: browser | case: debug low-level cards can be hidden with `off` | red: Chat Management still shows one-card-per-event debug output when Settings mode is `off`, or `off` is missing from Settings | green: Playwright sets AG-UI debug visibility to Off and proves low-level debug event cards are absent while user-facing AG-UI cards remain visible | interaction: open Settings General -> select Off -> open Chats -> send a mock message -> verify no debug event card rail labels and normal transcript remains | command_ref: CMD-FE-AGUI-BROWSER.
- AC-AGUI-02 | layer: browser | case: reasoning events split into one Thinking card per reasoning segment | red: multiple reasoning loops in the same run collapse into one card, separate cards leak each other's content, collapsed cards show no readable summary preview, work rail content renders after the assistant answer, or Thinking summaries omit child tool counts | green: Playwright proves one run renders a compact work rail before the assistant answer, the run contains two collapsed Thinking cards for two reasoning segments, the first summary includes `used 6 tools`, collapsed cards show concise preview text, and expanding each card reveals only that segment's streamed reasoning content | interaction: send mock chat message -> expand run work -> verify work rail appears before assistant bubble -> verify two Thinking cards -> verify collapsed preview text -> expand first card -> verify first segment content plus summary tool count -> expand second card -> verify second segment content only | command_ref: CMD-FE-AGUI-BROWSER.
- AC-AGUI-03 | layer: browser | case: tool events group into nested tool steps | red: tool start/args/activity/result/end events appear as separate top-level event cards, attach to the wrong Thinking card, or still render as card-within-card chat bubbles instead of compact work-rail steps | green: Playwright proves six tool steps render inside the first Thinking card only, the `Used tools` group header is visible, each step keeps action/target summary, expands to arguments, and completed state shows matching result content | interaction: send mock chat message -> expand first Thinking card -> verify `Used tools` header and tool step count -> expand one nested tool step -> verify args/result and no unmatched log leakage | command_ref: CMD-FE-AGUI-BROWSER.
- AC-AGUI-04 | layer: backend | case: protocol conversion includes stable tool fields | red: backend emits tool protocol events without stable `toolCallId`, without args, or with activity that cannot match a tool card | green: backend tests or focused inspection prove `TOOL_CALL_START`, `TOOL_CALL_ARGS`, `ACTIVITY_DELTA`, and `TOOL_CALL_RESULT` carry consistent `toolCallId` when tool context is known | command_ref: CMD-BE-TEST.

## Command Index

- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: TypeScript and production build pass.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: Rust backend compiles.
- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: green/verify/final | rerun: affected backend AG-UI protocol changes | expect: backend tests pass.
- CMD-FE-AGUI-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test chat-agui-user-cards.spec.ts --reporter=line | cost: slow/browser | when: red/green/verify | rerun: affected Chat Management AG-UI UI changes | expect: grouped work-rail reasoning cards, compact tool steps, and debug off behavior pass with concrete browser evidence.

## Evidence Rules

- Every `AC-AGUI-*` row must have final evidence in reports.
- Browser evidence must include the selected debug setting, send action, visible work rail, per-segment reasoning separation, compact nested tool steps, expansion behavior, assistant-after-work ordering, and negative assertion for low-level debug cards.
- Backend evidence must name the command and the fields used to match tool cards.

## Open Decisions

- None.
