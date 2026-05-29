# Acceptance: Chat AG-UI User Cards

## Acceptance Index

- AC-AGUI-01 [must] [interaction] Settings includes AG-UI debug mode `off`, and Chat Management hides low-level debug event cards when that mode is selected while preserving normal chat output; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AGUI-02 [must] [interaction] Chat Management groups reasoning start/content/end events into one default-collapsed user-facing Thinking card that can expand to show streamed reasoning content; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AGUI-03 [must] [interaction] Chat Management groups tool start/args/activity/result/end events into one default-collapsed user-facing tool card, summarizes action and target, shows arguments/logs while running, and shows result after completion; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AGUI-04 [must] [contract] Backend AG-UI protocol events provide stable tool-call ids, tool arguments, tool result content, and tool-id-matched activity fields when available; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
