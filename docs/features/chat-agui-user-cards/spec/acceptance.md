# Acceptance: Chat AG-UI User Cards

## Acceptance Index

- AC-AGUI-01 [must] [interaction] Settings includes AG-UI debug mode `off`, and Chat Management hides low-level debug event cards when that mode is selected while preserving normal chat output; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AGUI-02 [must] [interaction] Chat Management renders one default-collapsed user-facing Thinking card per completed or active reasoning segment in the latest run, presents those cards as a compact work rail ahead of the assistant answer, keeps a short summary preview after the segment ends, and ensures separate reasoning loops in one run expand independently without sharing streamed content; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AGUI-03 [must] [interaction] Chat Management groups tool start/args/activity/result/end events into compact tool steps, nests those steps under the latest reasoning segment in the same run when one exists, summarizes action and target, shows arguments/logs while running, and shows result after completion without rendering nested chat-style cards; proof: @ref:spec/test-plan.md#proof-rows.
- AC-AGUI-04 [must] [contract] Backend AG-UI protocol events provide stable tool-call ids, tool arguments, tool result content, and tool-id-matched activity fields when available; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
