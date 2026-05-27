# Test Report: React Agent Runtime Kernel

## Summary

Backend regression coverage proves the runtime kernel behavior and the shared event-bus foundation constraint.

## Evidence Rows

- AC-RAR-01 | red: `rtk make be-test` failed at compile because `domain::agent` was absent. | green: runtime isolation tests passed in `tron-backend/tests/agent_runtime.rs`. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-02 | red: model config could not create real provider request payloads. | green: request construction tests passed for all required model protocols. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-03 | red: tool execution had no allow-list boundary. | green: tool registry tests passed for allowed, missing, and unauthorized cases. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-04 | red: no ordered success lifecycle existed. | green: lifecycle tests passed for final-answer and tool-call flows. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-05 | red: no bounded failure lifecycle existed. | green: failure lifecycle tests passed for max-step, model, and tool failures. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-RAR-06 | red: event metadata initially exceeded the shared event-bus attr limit. | green: event payload safety tests passed after keeping run code in the event subject and attrs metadata-only. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/tests/agent_runtime.rs`
- `tron-backend/src/domain/agent.rs`

## Commands

- `rtk make be-test`
- `rtk make be-check`

## Blockers

None.
