# Feature Spec: EliteForge Memory Dreaming Script Foundation

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The recommended feature boundary is the script foundation only: create the repo-local skill source, implement the Codex-only CLI artifact workflow, preserve the IntentForge run/session/task contracts, and prove it with fixture tests plus Makefile-first validation. The production task prompts, Memory MCP, backend loading, UI, and multi-provider work stay out of this feature.

## Feature Identity And Context

- feature_slug: eliteforge-memory-dreaming-script-foundation
- feature_name: EliteForge Memory Dreaming Script Foundation
- status: approved_with_defaults
- owner: baiyihuan
- source_request: create `eliteforge-memory-dreaming` skill script foundation from `docs/brainstorm/eliteforge-memory-dreaming/prd.typ`
- main_agent: Codex lead agent on `feature/baiyihuan/memory/base`
- dependency_features: none found after scanning existing feature workspaces; this is a repo-local skill script source and does not depend on backend runtime features.
- dependency_contracts: `eliteforge-skill-spec`, `skill-creator` scaffold/validation, `eliteforge-git-feature-oriented-lite-spec`, root `Makefile`, IntentForge memory dreaming artifact contract, Codex local session JSONL format

## Requirement

Deliver the deterministic script foundation for the repo-local `eliteforge-memory-dreaming` skill. The feature owns the skill scaffold, Python CLI, Codex session import, `.intentforge-dreaming/` formal artifact root, `--artifact-root` development/test override, run/session artifacts, task state machine, temp output validation/promotion, schema-validated REM/Deep outputs without Memory MCP, deterministic report finalization, and tests.

## Product Source Slices

- brainstorming_source: `docs/brainstorm/eliteforge-memory-dreaming/2026-05-28-17-49-18.md`
- prd_feature_headings: `Feature: Memory Dreaming Skill Development - Script Foundation`
- prd_page_headings: `Page: Memory Dreaming CLI - Script Commands`, `Interaction: Memory Dreaming CLI - Initialize Codex Dream Run`, `Interaction: Memory Dreaming CLI - Execute Task Cycle`, `Interaction: Memory Dreaming CLI - Finalize Dream Run`, `Page: Memory Dreaming Artifact Workspace - Run And Session Files`
- prd_rule_headings: `Rule: Memory Dreaming Skill Development - Repo Local Skill Isolation`, `Rule: Memory Dreaming Skill Development - Artifact Root Resolution`, `Rule: Memory Dreaming Skill Development - Codex Session Privacy`, `Rule: Memory Dreaming Skill Development - Codex Provider Scope`, `Rule: Memory Dreaming Skill Development - Snapshot And Segment Contract`, `Rule: Memory Dreaming Skill Development - Normalized Event Contract`, `Rule: Memory Dreaming Skill Development - Session Scan Decisions`, `Rule: Memory Dreaming Skill Development - V1 Task Contract`, `Rule: Memory Dreaming Skill Development - Temp Output Promotion`, `Rule: Memory Dreaming Skill Development - Script Only Memory Boundary`, `Rule: Memory Dreaming Skill Development - Deterministic Finalization`, `Rule: Memory Dreaming Skill Development - Makefile First Verification`
- prd_exception_headings: `Exception: Memory Dreaming Skill Development - Unsupported Provider`, `Exception: Memory Dreaming Skill Development - Invalid Artifact Root`, `Exception: Memory Dreaming Skill Development - Malformed Codex Session`, `Exception: Memory Dreaming Skill Development - Invalid Task Transition`, `Exception: Memory Dreaming Skill Development - Invalid Temp Output`, `Exception: Memory Dreaming Skill Development - Finalization Blocked`, `Exception: Memory Dreaming Skill Development - Memory Integration Deferred`, `Exception: Memory Dreaming Skill Development - Runtime Loading Deferred`
- prd_acceptance_headings: `Acceptance: Memory Dreaming Skill Development - Script Foundation`, `Acceptance: Memory Dreaming Skill Development - Future Prompt And Memory Integration`

## Scope Boundary

### Assumptions

- Python standard library is sufficient for the script foundation unless implementation discovers a strong reason for a dependency.
- `skill-creator` remains the correct initializer for the skill scaffold, with output directed to the repo-local `tron-skills/` directory.
- Formal artifact resolution uses `.intentforge-dreaming/` under the current workspace.
- Automated tests and development smoke always pass `--artifact-root` pointing at a temporary or non-repo directory.
- Real `~/.codex` data may be used only for optional local smoke, not deterministic tests.

### In Scope

- `tron-skills/eliteforge-memory-dreaming/SKILL.md` and `agents/openai.yaml` with valid `eliteforge-*` metadata and minimal prompt guidance.
- Python CLI entry point `scripts/memory_dreaming.py` and focused internal modules under `scripts/memory_dreaming_lib/`.
- Codex provider for `~/.codex/sessions/**/*.jsonl`, `~/.codex/archived_sessions/*.jsonl`, and `~/.codex/session_index.jsonl`.
- Artifact root resolution, including `.intentforge-dreaming/` default and `--artifact-root` override.
- `runs/<dream-run-id>/` and `sessions/<session-key>/` artifact layout.
- Dreaming Session Snapshot creation, normalized events, segment metadata, and `segments/*.material.md`.
- V1 commands: `init-run`, `task-list`, `task-detail`, `task-start`, `task-complete`, `task-fail`, `task-retry`, `status`, and `finalize-run`.
- Task states `INIT`, `RUNNING`, `COMPLETED`, and `FAILED`, plus IntentForge V1 task types.
- Script-assigned `tempOutputRef`, schema validation, official artifact promotion, downstream task creation, and deterministic failure behavior.
- Agent-produced REM/Deep dry-run artifacts without Memory MCP, plus explicit Deep no-op only when a run has no imported sessions.
- Deterministic tests with fixtures and temporary artifact roots.
- Root Makefile target for script test validation.

### Out Of Scope

- Production task prompts for segment, session, Light, REM, or Deep analysis.
- Memory MCP reads or writes and Memory Core candidate or active persistence.
- Backend Skill Registry loading, MemoryDreamJob scheduling, API routes, database migrations, or UI pages.
- Claude Code, OpenCode, or IntentForge Session providers.
- Direct Java migration from the IntentForge repository.
- Automatic installation into local Codex runtime skill directories.

### Constraints

- Do not place the skill source under `.agents/skills` or any path loaded by `TRON_SKILLS_REGISTRY_ROOT`.
- Do not commit `.intentforge-dreaming/`, `/tmp/eliteforge-memory-dreaming`, temp outputs, or live Codex-derived runtime artifacts.
- CLI output must not print full message bodies.
- `task-complete` must read only the script-recorded `tempOutputRef`; it must not accept arbitrary official output paths or inline large JSON.
- `finalize-run` must be deterministic and must not call Memory MCP or accept an agent-authored report body.
- Root Makefile should be the first validation entry point for the script tests.

## Feature Model

### Actors

- Skill developer: develops and validates the repo-local skill source.
- Codex agent: calls the CLI and writes task outputs to assigned temp files.
- Dreaming script: owns artifact roots, run/session indexes, snapshots, tasks, schema validation, and final reports.
- Future MemoryDreamJob: later runtime caller that can use the same formal `.intentforge-dreaming/` contract.

### Core Objects

- Skill source: `tron-skills/eliteforge-memory-dreaming`.
- Artifact root: `.intentforge-dreaming/` by default, or `--artifact-root` override for tests and development smoke.
- DreamRun: one run under `runs/<dream-run-id>/`.
- Session work material: one normalized session under `sessions/<session-key>/`.
- Dreaming Session Snapshot: provider-neutral source manifest, session metadata, preprocessing metadata, normalized events, segments, and stats.
- DreamTask: stateful work item with task type, state, input refs, internal official output ref, and script-assigned `tempOutputRef`.
- Formal artifacts: segment notes, session summary, light memory points, REM governance, Deep governance, and dream report.

### State Flow

1. `init-run` resolves artifact root and creates a `dream-run-id`.
2. The Codex provider scans active sessions, archived sessions, and session index metadata.
3. Each candidate session receives a scan decision: import, reuse, resume, changed reimport, skip, or failed.
4. Imported or reimported sessions become snapshots and segment material files.
5. The script creates initial `SEGMENT_SUMMARY` tasks and records tasks in `tasks.json`.
6. `task-detail` exposes input refs, bounded preview, schema, and a script-owned `tempOutputRef`.
7. `task-start`, `task-complete`, `task-fail`, and `task-retry` enforce the V1 state machine.
8. Valid task completion promotes temp output to formal artifacts and creates downstream tasks.
9. REM and Deep complete through schema-validated agent outputs without Memory MCP; no-session runs use an explicit Deep no-op.
10. `status` reports blockers and `canFinalize`.
11. `finalize-run` writes or returns deterministic `dream-report.md` when finalization is allowed.

### Invariants

- The skill source is never auto-loaded from `.agents/skills`.
- Provider V1 accepts only `codex`.
- Formal runs default to `.intentforge-dreaming/`; development and tests use `--artifact-root`.
- `runs/<dream-run-id>/` does not copy session text.
- Session material and task artifacts stay under the resolved artifact root.
- Unknown provider-specific events degrade safely where possible.
- Invalid task transitions do not mutate official artifacts.
- Schema validation failure leaves the task `RUNNING`.
- Finalization is blocked by any `INIT`, `RUNNING`, or `FAILED` task, or incomplete Deep state.

## Page And Module Surface

UI: Not applicable.

- Skill source: `tron-skills/eliteforge-memory-dreaming/`.
- Skill metadata: `SKILL.md` and `agents/openai.yaml`.
- CLI entry point: `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py`.
- Internal script modules: `scripts/memory_dreaming_lib/codex_provider.py`, `artifacts.py`, `tasks.py`, `schemas.py`, `report.py`, and `time_utils.py`.
- Script tests and fixtures: `tron-skills/eliteforge-memory-dreaming/scripts/tests/`.
- Formal runtime artifact root: `.intentforge-dreaming/`.
- Development/test runtime artifact roots: temporary directories passed through `--artifact-root`.
- Validation entry: root `Makefile` target for skill memory dreaming tests.

## Repository Findings

- Root `AGENTS.md` requires `eliteforge-codex-superpower` for development and requires discussion before implementation.
- Root `AGENTS.md` says Makefile should be used first for build, test, and cleanup.
- `eliteforge-git-feature-oriented-lite-spec` is the declared git spec, and the active branch/worktree is `feature/baiyihuan/memory/base`.
- `docs/brainstorm/eliteforge-memory-dreaming/prd.typ` and `prd.pdf` now define the product-layer input for this feature.
- `docs/brainstorm/eliteforge-memory-dreaming/2026-05-28-17-49-18.md` records the approved artifact-root correction: formal `.intentforge-dreaming/`, override roots for development and tests.
- Existing backend Skill Registry reads configured registry paths, so this feature intentionally keeps source under `tron-skills/`.
- Root `Makefile` currently has frontend and backend targets but no skill script target.
- Strong feature dependencies: none found after scanning existing feature-spec.md files under `docs/features`; this is a new repo-local script feature.

## Open Decisions

- Common capability extraction: no candidate found. The artifact store, task runner, and Codex provider are local to this unfinished skill until prompt and backend adoption features prove stable reuse.
