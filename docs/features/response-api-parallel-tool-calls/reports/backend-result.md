# Backend Result

- status: completed
- ac: `AC-RPTC-01`, `AC-RPTC-02`
- changed:
  - Added `max_parallel_tool_calls` to internal-agent policy domain records, HTTP patch request parsing, invocation limits, SQLite persistence, and a forward migration.
  - Reworked parallel tool-call execution to batch by `min(remaining_total_budget, max_parallel_tool_calls, remaining_calls_in_turn)`.
  - Defaulted outbound tool-enabled model requests to `parallel_tool_calls: true`, while removing any runtime dependency on the provider response flag.
  - Preserved the existing max-tool-calls failure contract, but only after all remaining allowed calls in the run budget have executed.
- evidence:
  - `CMD-BE-CHECK`: passed
  - `rtk cargo test --test agent_runtime -- --nocapture`: passed
  - `rtk cargo test --test agent_management_api workspace_internal_agent_policy_limits_flow_into_runtime -- --nocapture`: passed
  - `CMD-BE-TEST`: passed
