# Test Report: Agent Run History

## Summary

Feature 3 backend persistence, query API, safety, and regression ACs have executable evidence.

## Evidence Rows

- AC-ARH-01 | red: `agent_run_history_api` first failed because no persisted run summary API existed | green: `agent_invocation_persists_safe_history_and_query_apis` reads completed history with run identity, timing, step count, observation count, and bounded output preview | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-ARH-02 | red: history list and detail routes returned no protected paged contract | green: authenticated list and detail assertions prove uniform body and paged records | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-ARH-03 | red: history safety assertions had no stored DTO to inspect | green: response JSON excludes prompt, completion text, raw tool output, API key, token, and request body values | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-ARH-04 | red: missing run detail did not expose stable `AGENT_RUN_NOT_FOUND` mapping | green: `missing_agent_run_history_returns_stable_error` proves stable code and uniform error body | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-ARH-05 | red: persistence wiring risked breaking runtime and invocation contracts | green: `rtk make be-check` and `rtk make be-test` pass with feature1, feature2, and history tests included | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/migrations/20260527_0017_create_agent_run_history.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/agent_repository.rs`
- `tron-backend/src/application/agent.rs`
- `tron-backend/src/domain/agent.rs`
- `tron-backend/tests/agent_run_history_api.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`

## Blockers

- None.
