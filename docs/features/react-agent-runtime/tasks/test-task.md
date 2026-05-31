# Test Task: React Agent Runtime Kernel

## Execution Contract

- feature_slug: react-agent-runtime
- status: done
- execution_mode: solo

## TDD Contract

Test rows verify the backend runtime evidence and feature document contract.

## Task Rows

- AC-RAR-01 | phase: verify | owner: lead | scope: backend runtime isolation regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-02 | phase: verify | owner: lead | scope: model request construction regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-03 | phase: verify | owner: lead | scope: tool registry regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-04 | phase: verify | owner: lead | scope: success lifecycle regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-05 | phase: verify | owner: lead | scope: failure lifecycle regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-06 | phase: verify | owner: lead | scope: event payload safety regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-07 | phase: verify | owner: lead | scope: system CLI runtime dispatch regression (`codex`/`opencode`) | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-08 | phase: verify | owner: lead | scope: explicit `AgentRuntime` orchestration (`start`/`resume`/`cancel`) and capability-routed adapter selection regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-09 | phase: verify | owner: lead | scope: canonical runtime event mapping and AGUI translation regression | command_ref: CMD-BE-TEST | status: done.
- AC-RAR-10 | phase: verify | owner: lead | scope: SQLite-backed canonical/AGUI replay persistence and post-restart replay regression | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Verification runs on the selected base worktree.

## Integration Notes

- Reuse `CMD-BE-TEST` evidence unless backend runtime files change after VERIFY.

## Open Decisions

- None.
