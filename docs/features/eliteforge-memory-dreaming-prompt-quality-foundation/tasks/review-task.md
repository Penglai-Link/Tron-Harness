# Execution Contract

owner: lead-reviewer
mode: lead
branch: feature/baiyihuan/memory/base
worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base

Review integrated worker changes against every acceptance row and record final evidence.

# Matrix Review Inputs

- AC-MDPQ-08 review must check FLOW-001..FLOW-055, task-boundary QUALITY rows, `source-coverage-audit.md`, and GAP-001..GAP-002 coverage.
- AC-MDPQ-09 review must check QUALITY-001..QUALITY-085, LTM-001..LTM-078, `source-coverage-audit.md`, and GAP-003..GAP-007 coverage.
- AC-MDPQ-10 review must check GOV-001..GOV-056, governance-critical QUALITY/LTM/FLOW rows, `source-coverage-audit.md`, and GAP-008..GAP-011 coverage.
- AC-MDPQ-11 and AC-MDPQ-12 review must check mandatory private Codex CLI evidence against FLOW/QUALITY/GOV/LTM source-backed quality rows and sanitized evidence rows before marking pass.
- AC-MDPQ-13 through AC-MDPQ-17 review must check the five quality-uplift findings from the private run: recall/compression, traceability, memory usability, evaluator strength, and private review rubric.

# TDD Contract

- Reuse worker evidence when fresh.
- Rerun affected Makefile-first commands after integration.
- Treat privacy leaks and accepted semantic mutants as release blockers.
- Treat uncovered blocker/high rows in `current-gap-matrix.md` as release blockers for AC-MDPQ-08 through AC-MDPQ-12.
- Treat uncovered blocker/high rows in `source-coverage-audit.md` as release blockers unless explicitly accepted out of scope by the user.
- Treat missing real-history Codex CLI evaluation evidence as a release blocker for AC-MDPQ-11 and AC-MDPQ-12 even if all synthetic tests pass.
- Treat missing upgraded real-history Codex CLI re-evaluation as a blocker for final quality claims after AC-MDPQ-13 through AC-MDPQ-17.

# Task Rows

- AC-MDPQ-01 | phase: review | owner: lead-reviewer | scope: integrated script contract | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-02 | phase: review | owner: lead-reviewer | scope: integrated schema and REM flow | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-03 | phase: review | owner: lead-reviewer | scope: integrated privacy controls | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-04 | phase: review | owner: lead-reviewer | scope: integrated evaluator and fixtures | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

- AC-MDPQ-05 | phase: review | owner: lead-reviewer | scope: integrated live-smoke dry-run helper | command_ref: CMD-LIVE-SMOKE-DRY | status: done

- AC-MDPQ-06 | phase: review | owner: lead-reviewer | scope: integrated skill validation | command_ref: CMD-SKILL-VALIDATE | status: done

- AC-MDPQ-07 | phase: review | owner: lead-reviewer | scope: integrated governance-only enforcement | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-08 | phase: review | owner: lead-reviewer | scope: integrated skill instructions and agent dreaming workflow | command_ref: CMD-SKILL-VALIDATE | status: done

- AC-MDPQ-09 | phase: review | owner: lead-reviewer | scope: integrated prompt quality constraints and semantic fixtures | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

- AC-MDPQ-10 | phase: review | owner: lead-reviewer | scope: integrated REM and Deep dry-run governance behavior | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-11 | phase: review | owner: lead-reviewer | scope: private real Codex CLI skill evaluation | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: done

- AC-MDPQ-12 | phase: review | owner: lead-reviewer | scope: sanitized private evaluation evidence and privacy leak check | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: done

- AC-MDPQ-13 | phase: review | owner: lead-reviewer | scope: bounded segmentation and compression coverage | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-14 | phase: review | owner: lead-reviewer | scope: claim-level EvidenceRef validation and evaluator checks | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: done

- AC-MDPQ-15 | phase: review | owner: lead-reviewer | scope: richer Light memory content and scoring | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

- AC-MDPQ-16 | phase: review | owner: lead-reviewer | scope: strengthened semantic fixture evaluator | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: done

- AC-MDPQ-17 | phase: review | owner: lead-reviewer | scope: upgraded private real-history quality rubric | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: prepared

# Worktree Notes

Review occurs only after worker branches merge into the base branch.

# Integration Notes

The lead agent owns conflict resolution, final reports, and command evidence.

# Open Decisions

- none
