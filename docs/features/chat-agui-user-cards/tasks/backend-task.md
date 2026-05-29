# Backend Task: Chat AG-UI Protocol Tool Fields

## Task Rows

| ac_refs | phase | owner | files | status |
| --- | --- | --- | --- | --- |
| AC-AGUI-04 | tdd | lead-agent | `tron-backend/src/application/agent/runtime.rs`, `tron-backend/src/application/agent/agui_stream.rs` | done |

## Notes

- Emit stable tool-call ids from runtime event attributes.
- Preserve existing AG-UI replay behavior.
- Avoid attributing activity logs to a tool unless the event carries the same tool-call id.
