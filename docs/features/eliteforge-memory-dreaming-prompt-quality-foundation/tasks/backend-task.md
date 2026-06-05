# Execution Contract

owner: backend-worker
mode: parallel
branch: feature/baiyihuan/memory/backend/script-contract
worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/backend/script-contract

Implement the script and prompt contract slice for the memory dreaming skill. Keep changes scoped to the assigned paths and do not overwrite parallel quality-evaluator edits.

# TDD Contract

- Start from failing tests for prompt metadata, V2 validation, evidence resolution, privacy defaults, and Memory MCP dry-run enforcement.
- Use `CMD-SKILL-MEMORY-DREAMING-TEST` as the primary proof command.
- Keep generated artifacts in temporary test directories.

# Task Rows

- AC-MDPQ-01 | phase: tdd | owner: backend-worker | scope: tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py, tron-skills/eliteforge-memory-dreaming/references | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-02 | phase: tdd | owner: backend-worker | scope: tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py, tron-skills/eliteforge-memory-dreaming/tests | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-03 | phase: tdd | owner: backend-worker | scope: tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py, .gitignore | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-07 | phase: tdd | owner: backend-worker | scope: tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py, tron-skills/eliteforge-memory-dreaming/tests | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

# Worktree Notes

This worker owns the main runtime contract. It must not edit `memory_dreaming_quality.py`, `memory_dreaming_live_smoke.py`, or quality fixture manifests except to keep imports compatible.

# Integration Notes

Expected integration conflict points are Makefile targets and shared tests. The lead agent owns final consolidation if both workers need the same test helper.

# Open Decisions

- none
