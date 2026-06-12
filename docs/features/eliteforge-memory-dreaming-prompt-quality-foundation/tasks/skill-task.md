# Execution Contract

owner: docs-worker
mode: parallel
branch: feature/baiyihuan/memory/docs/skill-quality-prompts
worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/docs/skill-quality-prompts

Implement the real-agent skill and prompt quality slice for the memory dreaming skill. Keep changes scoped to skill instructions, prompt references, evaluation references, prompt schemas, fixtures, and directly required prompt tests.

# Matrix Inputs

- AC-MDPQ-08 must cover `requirements-flow-matrix.md` FLOW-001..FLOW-055, `requirements-quality-matrix.md` QUALITY-071 and QUALITY-085, `source-coverage-audit.md` FLOW/QUALITY task-boundary rows, and `current-gap-matrix.md` GAP-001..GAP-002.
- AC-MDPQ-09 must cover `requirements-quality-matrix.md` QUALITY-001..QUALITY-085, `requirements-long-term-memory-matrix.md` LTM-001..LTM-078, `source-coverage-audit.md` QUALITY/LTM rows, and `current-gap-matrix.md` GAP-003..GAP-007.
- Matrix row ids are planning references; acceptance remains keyed only by AC-MDPQ-08 and AC-MDPQ-09.

# TDD Contract

- Start from failing prompt-quality fixtures or skill validation checks for real Codex Agent usage, evidence discipline, correction handling, scope boundaries, no-point reasoning, and memory pollution prevention.
- Add or update fixtures from the original source examples before claiming prompt quality; each new fixture group must cite the source line range from the matrix or `source-coverage-audit.md`.
- Use `CMD-SKILL-MEMORY-DREAMING-QUALITY` and `CMD-SKILL-VALIDATE` as primary proof commands.
- Do not read real `~/.codex` data in automated tests.
- Do not mark the skill high quality until `CMD-CODEX-CLI-SKILL-EVAL` later confirms a real private Codex Agent run.

# Task Rows

- AC-MDPQ-08 | phase: tdd | owner: docs-worker | scope: tron-skills/eliteforge-memory-dreaming/SKILL.md, tron-skills/eliteforge-memory-dreaming/references, tron-skills/eliteforge-memory-dreaming/tests | command_ref: CMD-SKILL-VALIDATE | status: done

- AC-MDPQ-09 | phase: tdd | owner: docs-worker | scope: tron-skills/eliteforge-memory-dreaming/references, tron-skills/eliteforge-memory-dreaming/quality-fixtures, tron-skills/eliteforge-memory-dreaming/tests | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

# Worktree Notes

This worker owns agent-facing instructions and prompt quality. It must not add a Codex CLI automation runner and must not treat deterministic fixture success as proof of real dreaming quality.

The skill entry can describe the global dreaming process, but it must require the Agent to advance through script commands (`init-run`, `task-list`, `task-detail`, `task-start`, temp output creation, `task-complete`, `task-fail` or `task-retry`, `status`, and `finalize-run`) until the run reaches a terminal state.

# Integration Notes

Expected integration conflict points are prompt reference files, quality fixtures, and skill validation. The lead agent owns final consolidation with the governance dry-run slice.

# Open Decisions

- none
