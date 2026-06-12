# Test Plan: EliteForge Memory Dreaming Script Foundation

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user confirmed AC scope after PRD and spec review
- approved_at: 2026-05-28

## Proof Rows

- AC-MDSF-01 | layer: regression | case: skill scaffold validation | red: skill path is absent or invalid, or scaffold is placed under runtime-loaded paths | green: skill-creator validation passes for `tron-skills/eliteforge-memory-dreaming` and path checks prove it is not under `.agents/skills` | command_ref: CMD-SKILL-VALIDATE.
- AC-MDSF-02 | layer: unit | case: artifact root resolution | red: commands require explicit artifact root or default to a test-only root | green: tests prove omitted `--artifact-root` resolves `.intentforge-dreaming/` and override roots isolate artifacts | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-03 | layer: unit | case: Codex fixture import and privacy | red: active or archived fixtures are ignored, snapshot shape is missing, or stdout includes message bodies | green: tests prove active/archived discovery, normalized snapshot and segment creation, and no full message body in stdout | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-04 | layer: unit | case: scan decision coverage | red: init-run cannot distinguish import, reuse, resume, changed reimport, skip, and failed session cases | green: tests assert all six scan decisions are written to `session-index.json` under fixture scenarios | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-05 | layer: unit | case: V1 task state machine | red: invalid task transitions mutate state or retry semantics are absent | green: tests prove start, complete, fail, retry, idempotent running start, and complete-before-start rejection | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-06 | layer: unit | case: temp output promotion | red: task-complete accepts arbitrary paths, bad JSON, or writes official artifacts before validation | green: tests prove recorded tempOutputRef validation, official artifact promotion, invalid output stays RUNNING, and downstream tasks appear | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-07 | layer: unit | case: dry-run governance tasks | red: REM or Deep requires Memory MCP or blocks script-only completion | green: tests prove agent-provided REM and Deep dry-run artifacts complete without Memory MCP, while no-session Deep can close through explicit no-op | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-08 | layer: unit | case: status and finalization | red: status allows finalization with open tasks or final report changes across repeated runs | green: tests prove blocker reporting, canFinalize readiness, deterministic report content, and idempotent finalize-run | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.
- AC-MDSF-09 | layer: regression | case: Makefile-first validation | red: root Makefile lacks a skill memory dreaming test target | green: `make skill-memory-dreaming-test` runs the deterministic script tests from the repository root | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST.

## Command Index

- CMD-SKILL-MEMORY-DREAMING-TEST | command: `make skill-memory-dreaming-test` | cost: medium | when: red-green-verify | rerun: after skill script, fixture, or Makefile changes | expect: deterministic skill script tests pass through Makefile.
- CMD-SKILL-VALIDATE | command: `test -d tron-skills/eliteforge-memory-dreaming && test ! -e .agents/skills/eliteforge-memory-dreaming && python3 /home/baiyihuan/.codex/skills/.system/skill-creator/scripts/quick_validate.py tron-skills/eliteforge-memory-dreaming` | cost: low | when: green-verify | rerun: after skill metadata or prompt files change | expect: skill path isolation and skill validation pass.
- CMD-DOCS-SPEC | command: `python3 /home/baiyihuan/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/eliteforge-memory-dreaming-script-foundation --phase spec --complete-spec` | cost: low | when: docs | rerun: after feature spec changes | expect: feature docs spec gate passes.

## Evidence Rules

- RED evidence must show the missing script behavior before GREEN.
- GREEN evidence must use deterministic fixtures and temporary artifact roots.
- Optional live `~/.codex` smoke may be recorded as supplemental evidence only; it cannot replace fixture tests.
- No browser evidence is required for this CLI/script feature.
- No Memory MCP evidence is expected in this feature.

## Open Decisions

- None.
