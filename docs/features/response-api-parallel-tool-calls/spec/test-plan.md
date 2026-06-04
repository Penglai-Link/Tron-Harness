# Test Plan: Response API Parallel Tool Calls

## Command Index

- CMD-BE-CHECK: `rtk make be-check`
- CMD-BE-TEST: `rtk make be-test`
- CMD-BE-RUNTIME: `rtk cargo test responses_stream_terminal_tool_calls_override_text_accumulator -- --nocapture`
- CMD-FE-CHECK: `rtk make fe-check`

## Proof Rows

| AC | case | red | green | command_ref |
| --- | --- | --- | --- | --- |
| AC-RPTC-01 | A tool-call response larger than the parallel cap is executed in sequential batches while preserving total-budget accounting. | Existing runtime fails the whole run before executing allowed calls when `calls.len() > maxToolCalls`. | Runtime batches calls by `maxParallelToolCalls` and remaining budget, then proceeds until calls are exhausted or budget is consumed. | CMD-BE-TEST |
| AC-RPTC-02 | A run with remaining total budget smaller than the current tool-call set executes only the remaining allowed calls and then fails on the first excess call. | Existing runtime rejects the entire set before any partial progress. | Runtime allows partial progress up to budget and surfaces the existing max-tool-calls failure contract for the first excess call. | CMD-BE-TEST |
| AC-RPTC-03 | Internal-agent policy UI shows separate controls for total tool-call budget and parallel batch cap. | Existing UI exposes only one ambiguous `Max tool calls` field. | UI exposes both values with clarified help text and compatible API payload normalization. | CMD-FE-CHECK |
