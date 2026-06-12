# Execution Contract

owner: lead-reviewer
mode: lead
branch: feature/baiyihuan/memory/base
worktree: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base

This task is the mandatory final skill-quality gate. Run the private real Codex CLI evaluation only after implementation is ready and the user approves the exact prompt, sampled data scope, artifact root, redaction policy, and quality rubric. The current preparation stage records the executable protocol; it must not read real Codex history yet. This task must not be delegated to a worker because it touches private historical Codex data and produces sensitive temporary artifacts.

# Matrix Inputs

- AC-MDPQ-11 must cover `requirements-flow-matrix.md` FLOW-001..FLOW-055, `requirements-quality-matrix.md` QUALITY-071..QUALITY-085, `requirements-long-term-memory-matrix.md` LTM-001..LTM-078, `source-coverage-audit.md` private-eval relevant rows, and `current-gap-matrix.md` GAP-012.
- AC-MDPQ-12 must cover privacy-sensitive rows from `requirements-flow-matrix.md` FLOW-006, FLOW-017, FLOW-021, FLOW-022, FLOW-044..FLOW-055; `requirements-quality-matrix.md` QUALITY-012, QUALITY-017, QUALITY-048, QUALITY-059, QUALITY-071..QUALITY-085; `requirements-governance-matrix.md` GOV-049..GOV-056; `requirements-long-term-memory-matrix.md` LTM-002, LTM-010, LTM-024..LTM-026, LTM-031..LTM-078; `source-coverage-audit.md` privacy/private-eval rows; and `current-gap-matrix.md` GAP-013..GAP-014.
- Matrix row ids are planning references; acceptance remains keyed only by AC-MDPQ-11 and AC-MDPQ-12.

# Verification Contract

- Use `CMD-CODEX-CLI-SKILL-EVAL` only after explicit user approval for that exact run.
- The proposed V1 run is recorded in `tron-skills/eliteforge-memory-dreaming/references/private-evaluation.md#proposed-ac11ac12-run-v1`; it is not approved for execution until the user confirms that exact prompt and command.
- The real Codex CLI session must instruct the Agent to use `tron-skills/eliteforge-memory-dreaming`; it must not rely on a separate automation runner that replaces the Agent.
- The evaluation must use local real Codex conversation history from `$CODEX_HOME`, `~/.codex`, or another user-approved Codex history root; synthetic fixtures and helper-script dry runs cannot satisfy AC-MDPQ-11 or AC-MDPQ-12.
- Keep artifacts outside the repository and commit only sanitized aggregate quality evidence.
- The private review must judge extraction quality, governance reasoning, structure placement, and privacy redaction against the matrix rows, not only whether scripts completed.
- The private review record must follow LTM-078: core/extension scenario counts, key failure ids, redaction status, reviewer conclusion, and no raw private content.

# Task Rows

- AC-MDPQ-11 | phase: verify | owner: lead-reviewer | scope: private Codex CLI session, repo-external artifact root, sanitized review notes | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: done

- AC-MDPQ-12 | phase: verify | owner: lead-reviewer | scope: sanitized evidence report, privacy leak check, committed docs only | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: done

# Worktree Notes

This verification happens from the base worktree after implementation branches merge. No raw `~/.codex` session material, source ids, source hashes, private absolute paths, or raw Agent dreaming outputs may be committed.

# Integration Notes

If the private evaluation is not run, the skill is not complete. If it finds weak extraction quality, stale memory pollution, unsupported governance decisions, or privacy leaks, reopen the relevant AC slice before claiming completion.

# Proposed Run V1

- history_root: `${CODEX_HOME:-$HOME/.codex}`
- sample_scope: 8 real Codex sessions prepared through `memory_dreaming_live_smoke.py` symlink sampling with seed `20260529`
- sample_preparation_root: `/tmp/eliteforge-memory-dreaming-private-eval/20260529-ac11-ac12-v1/scope`
- dreaming_artifact_root: `/tmp/eliteforge-memory-dreaming-private-eval/20260529-ac11-ac12-v1/dreaming`
- dream_run_id: `ac11-ac12-real-codex-20260529-v1`
- execution_model: real `codex exec` Agent session; helper scripts prepare sample scope, enforce task state, validate JSON, and generate deterministic reports only
- review_output: `SANITIZED_AC11_AC12_RESULT` aggregate counts, failure buckets, redaction status, privacy leak status, AC11/AC12 result, and sanitized reviewer conclusion only

# Pending Approval Items

- exact command and prompt in `tron-skills/eliteforge-memory-dreaming/references/private-evaluation.md#proposed-ac11ac12-run-v1`
- sample size and seed
- artifact root under `/tmp`
- redaction policy and forbidden committed fields
- PASS/FAIL rubric for AC-MDPQ-11 and AC-MDPQ-12

# Open Decisions

- none

# Evidence

- AC-MDPQ-11 | green: `CMD-CODEX-CLI-SKILL-EVAL` completed with exit code 0; sampled 8 real Codex sessions; completed 34 tasks: Segment 8, Session 8, Light 8, REM 9, Deep 1; generated deterministic dream report | report: `reports/private-evaluation-report.md` | status: done.
- AC-MDPQ-12 | green: sanitized result returned PASS for privacy leak check and redaction status; committed evidence contains aggregate counts only; raw private artifacts remain outside the repository | report: `reports/private-evaluation-report.md` | status: done.
