# Test Task: EliteForge Memory Dreaming Script Foundation

## Execution Contract

- feature_slug: eliteforge-memory-dreaming-script-foundation
- status: ready
- execution_mode: hybrid
- canonical_report_target: reports/test-report.md

## TDD Contract

Use bounded AC slices with RED before GREEN and focused VERIFY. The first slice creates the skill scaffold and Makefile target; importer and workflow slices start only after the scaffold slice is integrated into the base branch.

## Task Rows

- AC-MDSF-01 | phase: tdd | owner: tdd-slice-worker | scope: `tron-skills/eliteforge-memory-dreaming/SKILL.md`, `agents/openai.yaml`, initial script directory, and runtime-loaded path isolation checks | command_ref: CMD-SKILL-VALIDATE | status: done.
- AC-MDSF-09 | phase: tdd | owner: tdd-slice-worker | scope: root `Makefile` skill memory dreaming test target and deterministic test command wiring | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-02 | phase: tdd | owner: tdd-slice-worker | scope: CLI artifact root resolution for `.intentforge-dreaming/` default and `--artifact-root` override | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-03 | phase: tdd | owner: tdd-slice-worker | scope: Codex active and archived fixture discovery, snapshot normalization, segment material generation, and stdout privacy | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-04 | phase: tdd | owner: tdd-slice-worker | scope: `init-run` session index scan decisions for import, reuse, resume, changed reimport, skip, and failed fixture cases | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-05 | phase: tdd | owner: tdd-slice-worker | scope: V1 task state machine commands and invalid transition handling | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-06 | phase: tdd | owner: tdd-slice-worker | scope: recorded `tempOutputRef` validation, official artifact promotion, schema failure behavior, and downstream task creation | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-07 | phase: tdd | owner: tdd-slice-worker | scope: agent-provided REM and Deep dry-run artifacts without Memory MCP dependency, with no-session Deep no-op only | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.
- AC-MDSF-08 | phase: tdd | owner: tdd-slice-worker | scope: `status` blockers, `canFinalize`, deterministic `dream-report.md`, and idempotent `finalize-run` | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done.

## Worktree Notes

- AC-MDSF-01 and AC-MDSF-09 map to `WT-DOCS` in `orchestration/worktree-plan.md`.
- AC-MDSF-02 through AC-MDSF-04 map to `WT-CODEX` after scaffold integration.
- AC-MDSF-05 through AC-MDSF-08 map to `WT-TASKS` after scaffold integration.
- Workers must use selected-git-skill worktrees and must not edit the base worktree directly.

## Integration Notes

- Scaffold and Makefile changes establish the shared files that later workers depend on.
- Importer and workflow workers should receive the accepted AC rows, command aliases, write scope, and current base branch path.
- Deterministic tests must use fixture sessions and temporary artifact roots.
- Optional live `~/.codex` smoke is supplemental only and must use a non-repo artifact root.

## Open Decisions

- None.
