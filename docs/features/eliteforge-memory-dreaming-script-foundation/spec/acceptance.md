# Acceptance: EliteForge Memory Dreaming Script Foundation

## Acceptance Index

- AC-MDSF-01 [must] [business] Repo-local skill scaffold is valid and remains outside runtime-loaded skill paths; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-02 [must] [business] CLI resolves `.intentforge-dreaming/` by default and supports isolated `--artifact-root` overrides; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-03 [must] [business] Codex provider imports active and archived fixtures into normalized snapshots and segments without stdout message leakage; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-04 [must] [business] `init-run` records import, reuse, resume, changed reimport, skip, and failed scan decisions; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-05 [must] [business] V1 task commands enforce valid state transitions, retry, and invalid transition rejection; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-06 [must] [business] `task-complete` validates recorded `tempOutputRef`, promotes official artifacts, and creates downstream tasks; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-07 [must] [business] REM and Deep complete through schema-validated dry-run artifacts without Memory MCP calls, with no-session Deep no-op limited to empty runs; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-08 [must] [business] `status` blocks incomplete runs and `finalize-run` idempotently writes deterministic reports; proof: @ref:spec/test-plan.md#proof-rows.
- AC-MDSF-09 [must] [regression] Root Makefile exposes and runs the skill script test target; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
