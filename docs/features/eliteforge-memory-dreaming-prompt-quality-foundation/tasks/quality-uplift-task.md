# Execution Contract

owner: lead
mode: approved integrated implementation
branch: feature/baiyihuan/memory/base
worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base

Implement the approved five-point quality uplift after real-history evaluation showed thin memory content and weak traceability.

# Task Rows

- AC-MDPQ-13 | phase: tdd | owner: lead | scope: `codex_provider.py`, task validation, prompts, schemas, tests | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-14 | phase: tdd | owner: lead | scope: EvidenceRef validation, quality evaluator, prompts, schemas, fixtures, tests | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-15 | phase: tdd | owner: lead | scope: Light point schema, prompts, quality rubric, fixtures, evaluator, tests | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

- AC-MDPQ-16 | phase: tdd | owner: lead | scope: quality evaluator and semantic fixtures | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

- AC-MDPQ-17 | phase: spec | owner: lead | scope: private evaluation rubric and sanitized report expectations | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: prepared

# Notes

- The user explicitly approved doing all five quality-uplift points before implementation.
- This slice is integrated in the base worktree because the five points intentionally touch shared prompt, schema, validator, evaluator, and fixture contracts.
- No real private Codex history is committed. Future private re-evaluation must use repo-external artifacts and sanitized aggregate evidence only.
