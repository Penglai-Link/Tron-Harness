# Execution Contract

owner: backend-worker
mode: parallel
branch: feature/baiyihuan/memory/backend/governance-dry-run
worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/backend/governance-dry-run

Implement the no-Memory-MCP REM and Deep dry-run governance slice. Keep changes scoped to the script governance contract, schemas, prompt requirements, and tests needed to prove non-empty governance behavior.

# Matrix Inputs

- AC-MDPQ-10 must cover `requirements-governance-matrix.md` GOV-001..GOV-056.
- AC-MDPQ-10 must also cover governance-critical rows from `requirements-quality-matrix.md` QUALITY-050..QUALITY-085, `requirements-flow-matrix.md` FLOW-020..FLOW-055, `requirements-long-term-memory-matrix.md` LTM-014..LTM-078, `source-coverage-audit.md` GOV/Deep rows, and `current-gap-matrix.md` GAP-008..GAP-011.
- Matrix row ids are planning references; acceptance remains keyed only by AC-MDPQ-10.

# TDD Contract

- Start from failing tests where REM or Deep returns a deterministic no-op despite merge, conflict, stale, promotion, reject, or authorization work.
- Add tests for no-MCP dry-run without fabricated card/timeline data, proposed operation structure, authorization evidence, MCP boundary bypass, stale revision recovery, and conflict/stale classification.
- Use `CMD-SKILL-MEMORY-DREAMING-TEST` as the primary proof command.
- Keep V1 Memory MCP call count at zero and reject any agent output that claims real Memory MCP writes.

# Task Rows

- AC-MDPQ-10 | phase: tdd | owner: backend-worker | scope: tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py, tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_lib, tron-skills/eliteforge-memory-dreaming/references, tron-skills/eliteforge-memory-dreaming/tests | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

# Worktree Notes

This worker owns governance mechanics. It must not weaken privacy controls, evidence resolution, or the V2 task completion contract from the earlier script foundation.

No Memory MCP availability does not make REM or Deep a no-op. The expected output is dry-run governance intent: read intents, write intents, proposed memory operations, decision reasons, authorization requirements, and escalation reasons, with no executed MCP writes.

# Integration Notes

Expected integration conflict points are schemas and task-complete validation. The lead agent resolves conflicts against AC-MDPQ-08 and AC-MDPQ-09 prompt contracts.

# Evidence

- AC-MDPQ-10 | red: backend-worker RED showed legacy REM/Deep governance outputs were accepted without `executionState`, structured proposed operations, escalation reasons, Deep request coverage, or fake-execution rejection | green: integrated `make skill-memory-dreaming-test` passed with 34 tests after MCP-ready dry-run governance validation | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDPQ-10 | verify: integrated `make skill-memory-dreaming-quality-test` passed with 10 tests and `CMD-SKILL-VALIDATE` passed; no real Memory MCP writes, no real Codex history evaluation, and no private artifacts were produced | status: done.

# Open Decisions

- none
