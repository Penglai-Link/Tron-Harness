= PRD: EliteForge Memory Dreaming Skill Script Foundation

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: repo-local `eliteforge-memory-dreaming` skill source for Tron development.
- Problem: the IntentForge memory dreaming design has a workflow and artifact model, but Tron does not yet have a deterministic script foundation that can safely import Codex sessions, manage dreaming tasks, validate agent outputs, and produce a run report before final skill prompts are designed.
- Target users: Tron skill developers, Codex-based implementation agents, future MemoryDreamJob maintainers, and future memory-governance operators.
- Outcome: the repository contains an isolated skill source under `tron-skills/eliteforge-memory-dreaming` with a tested Python CLI and artifact workflow that uses `.intentforge-dreaming/` for formal runs while allowing isolated artifact roots for development tests and local Codex smoke.

== Background
- Why now: the IntentForge memory dreaming workflow is already designed, and this project needs a stable script layer before investing in prompt quality, Memory MCP integration, backend jobs, or UI surfaces.
- Current pain: incomplete skills must not be loaded by Tron agents, development and automated tests must not pollute formal `.intentforge-dreaming/` runtime artifacts, and no current script implements the V1 dreaming command set, Codex import, task state machine, or finalization gate.
- Business context: this work creates the local execution substrate for later memory extraction, REM governance, Deep governance, and MemoryDreamJob integration while keeping long-term Memory Core writes out of this first slice.

== Goals
- Create the new skill source at `tron-skills/eliteforge-memory-dreaming` without placing it under `.agents/skills` or a configured runtime registry path.
- Keep `SKILL.md` and `agents/openai.yaml` valid for an `eliteforge-*` skill, while leaving final task prompts minimal until the script layer is proven.
- Provide one stable Python CLI entry point, `scripts/memory_dreaming.py`, backed by focused internal modules.
- Support V1 `provider=codex` for active sessions, archived sessions, and session index metadata.
- Use `.intentforge-dreaming/` as the formal default artifact root, and support `--artifact-root` as a development, test, and smoke-run override.
- Preserve the IntentForge V1 command set, task states, scan decisions, artifact layout, snapshot boundaries, `tempOutputRef` promotion rule, and deterministic finalization behavior.
- Cover the script layer with deterministic tests that use fixtures and temporary directories, plus an optional live Codex smoke path that does not commit runtime artifacts.
- Add a Makefile-first validation entry point for the skill script tests so project verification follows repository convention.

== Non-Goals
- Do not design production task prompts for segment, session, Light, REM, or Deep analysis in this slice.
- Do not call Memory MCP or write candidate, active, archived, or expired Memory Core records.
- Do not install the skill into the local Codex runtime or register it with the Tron backend Skill Registry.
- Do not add backend HTTP APIs, scheduled jobs, database schema, or frontend pages.
- Do not implement Claude Code, OpenCode, or IntentForge Session providers.
- Do not migrate IntentForge Java code directly.

== Scope
- In scope: skill source scaffold, minimal skill metadata, Python CLI, Codex JSONL reader, `.intentforge-dreaming/` artifact workspace, `--artifact-root` override for development and tests, Dreaming Session Snapshot generation, segment material files, run/session indexes, V1 task lifecycle, schema validation, temporary output promotion, dry-run or no-op REM/Deep task support, deterministic report generation, fixture tests, and a Makefile target for script tests.
- Out of scope: final prompt contracts, Memory MCP reads or writes, candidate or active memory persistence, Tron backend Skill Registry loading, MemoryDreamJob scheduling, frontend UI, multi-provider support, direct Java migration, and automatic installation into the local Codex skill runtime.

== Feature Requirements
=== Module: Memory Dreaming Skill Development
==== Feature: Memory Dreaming Skill Development - Script Foundation
- name: Repo-local memory dreaming script foundation.
- description: create an isolated skill source with a tested CLI that imports Codex sessions into the formal `.intentforge-dreaming/` artifact root or an explicit development/test override root, manages V1 dreaming tasks, validates task output from temp files, and finalizes runs deterministically.
- user or role: Codex agent, Tron skill developer, and future MemoryDreamJob orchestrator.
- entry point: `python3 tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py`.
- trigger: a developer or agent explicitly invokes `init-run` with `provider=codex`, optionally passing `--artifact-root` for development or test isolation.
- preconditions: the repository is on a feature branch, formal runs may use `.intentforge-dreaming/`, and deterministic tests use fixture sessions with isolated temporary artifact roots.
- main flow: scaffold the skill source, run `init-run`, scan Codex sessions, write run/session indexes, normalize matching sessions into snapshots, materialize segments, execute task lifecycle commands, promote validated temp outputs, run or mark no-op REM/Deep, expose final status, and generate `dream-report.md`.
- inputs and outputs: Codex JSONL files and CLI arguments in; `.intentforge-dreaming/runs/<dream-run-id>/...` and `.intentforge-dreaming/sessions/<session-key>/...` by default, or the same structure under an override `<artifact-root>` for development/tests, plus structured command output and tests.
- permissions or visibility: no automatic runtime loading; no Memory Core writes; CLI output must not print full message bodies.
- related page headings: `Page: Memory Dreaming CLI - Script Commands`, `Page: Memory Dreaming Artifact Workspace - Run And Session Files`.
- related business rule headings: `Rule: Memory Dreaming Skill Development - Repo Local Skill Isolation`, `Rule: Memory Dreaming Skill Development - Artifact Root Resolution`, `Rule: Memory Dreaming Skill Development - Codex Session Privacy`, `Rule: Memory Dreaming Skill Development - V1 Task Contract`, `Rule: Memory Dreaming Skill Development - Temp Output Promotion`, `Rule: Memory Dreaming Skill Development - Deterministic Finalization`.
- related exception headings: `Exception: Memory Dreaming Skill Development - Unsupported Provider`, `Exception: Memory Dreaming Skill Development - Invalid Artifact Root`, `Exception: Memory Dreaming Skill Development - Malformed Codex Session`, `Exception: Memory Dreaming Skill Development - Invalid Task Transition`, `Exception: Memory Dreaming Skill Development - Invalid Temp Output`, `Exception: Memory Dreaming Skill Development - Finalization Blocked`.

==== Feature: Memory Dreaming Skill Development - Prompt And Memory Integration
- name: future prompt and Memory MCP integration.
- description: add production prompt contracts and Memory MCP-backed REM/Deep governance after the script foundation is stable.
- user or role: memory-governance agent and future backend job runner.
- entry point: future `SKILL.md` instructions and task prompt references.
- trigger: after script tests and artifact semantics are stable.
- preconditions: the script foundation has validated task schemas, artifact promotion, status, and finalization.
- main flow: replace placeholder task guidance with production prompts, allow authorized Memory MCP reads in REM/Deep, and produce candidate governance outputs for later persistence.
- inputs and outputs: script task details and existing Memory data in; validated governance artifacts out.
- permissions or visibility: high-impact Memory changes remain outside this first script feature.
- related page headings: none for this slice.
- related business rule headings: `Rule: Memory Dreaming Skill Development - Script Only Memory Boundary`.
- related exception headings: `Exception: Memory Dreaming Skill Development - Memory Integration Deferred`.

==== Feature: Memory Dreaming Skill Development - Backend And Runtime Adoption
- name: future Tron backend and runtime adoption.
- description: make the proven skill available to Tron backend jobs or runtime registries only after it is complete enough to load safely.
- user or role: backend service, workspace administrator, and future scheduled MemoryDreamJob.
- entry point: future backend Skill Registry or job orchestration.
- trigger: after script foundation and prompt integration are stable.
- preconditions: skill source has passed validation, artifact paths are configured, and runtime loading is explicitly enabled.
- main flow: register or install the skill, schedule or trigger dream runs, monitor outputs, and connect candidate memory governance to approved persistence flows.
- inputs and outputs: backend job config and skill registry metadata in; scheduled runs and reviewable artifacts out.
- permissions or visibility: no backend loading in this slice.
- related page headings: none for this slice.
- related business rule headings: `Rule: Memory Dreaming Skill Development - Repo Local Skill Isolation`.
- related exception headings: `Exception: Memory Dreaming Skill Development - Runtime Loading Deferred`.

== Page Prototypes And Interactions
=== Page: Memory Dreaming CLI - Script Commands
- entry: `python3 tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py <command> ...`; development and tests may add `--artifact-root <path>`.
- layout: command-line interface with JSON-oriented output for agents and concise human-readable errors.
- data display: run id, session counts, task summaries, task details, status summary, artifact refs, validation errors, and report refs.
- actions: `init-run`, `task-list`, `task-detail`, `task-start`, `task-complete`, `task-fail`, `task-retry`, `status`, and `finalize-run`.
- states: initialized, task pending, task running, task completed, task failed, ready to finalize, completed, and blocked.

==== Interaction: Memory Dreaming CLI - Initialize Codex Dream Run
- trigger control: `init-run --provider codex --started-after <time> --ended-before <time> --max-sessions <n>`, optionally with `--artifact-root <path>` for development or tests.
- preconditions: resolved artifact root is writable; Codex fixture or local session paths are available; provider is `codex`.
- button or control behavior: not applicable.
- form rules: provider must be supported; time range and max session bounds must be valid.
- confirmation behavior: no confirmation prompt in V1; formal runs use `.intentforge-dreaming/`, while development/test callers choose an override root explicitly.
- success feedback: command returns a run id, scope ref, session index ref, imported/reused/skipped counts, and first tasks.
- failure feedback: command returns a typed error without printing full session text.
- navigation or refresh behavior: subsequent commands address the same run through the resolved artifact root and `--run-id`.

==== Interaction: Memory Dreaming CLI - Execute Task Cycle
- trigger control: `task-detail`, `task-start`, agent writes `tempOutputRef`, and `task-complete`.
- preconditions: task exists; task is `INIT` before first start or `RUNNING` before complete; temp output path is assigned by the script.
- button or control behavior: not applicable.
- form rules: agent output must match the task schema and be read from the recorded `tempOutputRef`.
- confirmation behavior: no direct official artifact write by the agent.
- success feedback: official artifact is written under `.intentforge-dreaming/sessions/...` or `.intentforge-dreaming/runs/...` by default, or the same paths under an override root, and downstream tasks are created or enabled.
- failure feedback: invalid output leaves the task `RUNNING`; failed tasks can be retried through `task-retry`.
- navigation or refresh behavior: `task-list` and `status` reflect the updated task graph.

==== Interaction: Memory Dreaming CLI - Finalize Dream Run
- trigger control: `status` followed by `finalize-run`.
- preconditions: no tasks are `INIT`, `RUNNING`, or `FAILED`; Deep is complete or explicitly no-op; run is not already finalized.
- button or control behavior: not applicable.
- form rules: `finalize-run` accepts only run identity and reads formal artifacts.
- confirmation behavior: no Memory MCP call and no agent-written report body.
- success feedback: command returns `dream-report.md` ref and marks the run completed.
- failure feedback: command returns blocker reasons and leaves the run unfinalized.
- navigation or refresh behavior: repeated `finalize-run` returns the existing report ref.

=== Page: Memory Dreaming Artifact Workspace - Run And Session Files
- entry: `.intentforge-dreaming/runs/<dream-run-id>/` and `.intentforge-dreaming/sessions/<session-key>/`; development and tests may resolve the same paths under `<artifact-root>/`.
- layout: IntentForge-compatible `runs/` and `sessions/` views under the formal `.intentforge-dreaming/` root or an override root.
- data display: `dream-scope.json`, `session-index.json`, `tasks.json`, `snapshot.json`, `segments/*.material.md`, `segment-notes/*.notes.json`, `session-summary.json`, `light-memory-points.json`, `rem-governance.json`, `deep-governance.json`, and `dream-report.md`.
- actions: scripts create, validate, update, reuse, and finalize artifacts.
- states: fixture test root, manual live smoke root, reusable completed session material, incomplete session material, and finalized run.

== Business Rules
=== Rule: Memory Dreaming Skill Development - Repo Local Skill Isolation
- condition: the skill source is created or modified in Tron.
- decision or calculation: place source under `tron-skills/eliteforge-memory-dreaming` and do not copy it to `.agents/skills` or a configured runtime skill registry.
- state transition: the skill remains development source until a future explicit installation or backend adoption task.
- permission or data boundary: incomplete skill behavior must not affect Tron agents.
- conflict or priority rule: user-selected source isolation overrides generic local Codex skill installation defaults.
- example: backend skill discovery must not load `tron-skills/eliteforge-memory-dreaming` during this slice.

=== Rule: Memory Dreaming Skill Development - Artifact Root Resolution
- condition: any CLI command reads or writes dreaming artifacts.
- decision or calculation: when `--artifact-root` is omitted, resolve artifacts to `.intentforge-dreaming/` under the current workspace; when provided, use the explicit override root for development, tests, or smoke runs.
- state transition: run and session files are created under the resolved root.
- permission or data boundary: deterministic tests and development smoke must not write to formal `.intentforge-dreaming/`; formal runs may write there by design.
- conflict or priority rule: explicit CLI path wins over the `.intentforge-dreaming/` default.
- example: `init-run --provider codex` writes under `.intentforge-dreaming/`, while `init-run --artifact-root /tmp/eliteforge-memory-dreaming --provider codex` writes under `/tmp`.

=== Rule: Memory Dreaming Skill Development - Codex Session Privacy
- condition: the Codex provider imports or previews local session data.
- decision or calculation: normalize into snapshots and material files while command output avoids full message bodies.
- state transition: session hashes, refs, summaries, task refs, and bounded previews may appear in command output.
- permission or data boundary: full conversation text stays in artifact files under the resolved root and runtime artifacts are not committed.
- conflict or priority rule: deterministic fixture tests cannot depend on real `~/.codex` data.
- example: `task-detail` returns `materialRef` and a preview instead of embedding a full segment in stdout.

=== Rule: Memory Dreaming Skill Development - Codex Provider Scope
- condition: `init-run` runs with `provider=codex`.
- decision or calculation: scan `~/.codex/sessions/**/*.jsonl`, `~/.codex/archived_sessions/*.jsonl`, and `~/.codex/session_index.jsonl` when using live local data.
- state transition: matching sessions receive scan decisions and may become snapshots.
- permission or data boundary: only `provider=codex` is accepted in V1.
- conflict or priority rule: provider identity is a run source identity, not a memory scope.
- example: `provider=claude` fails as unsupported in this slice.

=== Rule: Memory Dreaming Skill Development - Snapshot And Segment Contract
- condition: a matching Codex session is imported or reimported.
- decision or calculation: write `snapshot.json` with `sourceManifest`, `session`, `preprocess`, `events`, `segments`, and `stats`, then generate `segments/*.material.md`.
- state transition: imported sessions become reusable session work material.
- permission or data boundary: snapshots are working materials, not Memory Core records.
- conflict or priority rule: segment material can be regenerated from the snapshot.
- example: a task for `SEGMENT_SUMMARY` receives a material ref derived from `snapshot.json`.

=== Rule: Memory Dreaming Skill Development - Normalized Event Contract
- condition: Codex JSONL rows are transformed into snapshot events.
- decision or calculation: normalize events into stable order, timestamp, event type, role, text, tool, files, trust boundary, raw ref, hash, and redaction metadata where available.
- state transition: normalized events become the only source for generated segment material.
- permission or data boundary: raw provider differences stay inside the importer and do not leak into task contracts.
- conflict or priority rule: unknown or provider-specific rows map to safe `OTHER` or equivalent values instead of breaking the whole run when possible.
- example: user messages, assistant messages, tool calls/results/errors, system events, compaction, subagent, notification, and task events map to the source design's event categories.

=== Rule: Memory Dreaming Skill Development - Session Scan Decisions
- condition: `init-run` sees a candidate session.
- decision or calculation: write one of `IMPORT_NEW`, `REUSE_COMPLETE`, `RESUME_INCOMPLETE`, `REIMPORT_CHANGED`, `SKIP`, or `FAILED` to `session-index.json`.
- state transition: decisions control whether snapshots and tasks are created, reused, resumed, or reported as failed.
- permission or data boundary: no CLI option exposes forced reimport or low-level budget tuning in V1.
- conflict or priority rule: script-owned decisions keep agent prompts simple and reproducible.
- example: unchanged completed session material can be reused instead of reimported.

=== Rule: Memory Dreaming Skill Development - V1 Task Contract
- condition: the script creates or updates tasks.
- decision or calculation: use only task states `INIT`, `RUNNING`, `COMPLETED`, and `FAILED`, and task types `SEGMENT_SUMMARY`, `SESSION_SUMMARY`, `LIGHT_MEMORY_POINTS`, `REM_GOVERN_MEMORY_POINT`, and `DEEP_MEMORY_GOVERNANCE`.
- state transition: `task-start` moves `INIT` to `RUNNING`, `task-complete` moves valid `RUNNING` to `COMPLETED`, `task-fail` moves `RUNNING` to `FAILED`, and `task-retry` moves `FAILED` to `INIT`.
- permission or data boundary: no `task-checkpoint`, `task-skip`, `claim-task`, `write-artifact`, or `register-session` exists in V1.
- conflict or priority rule: large work should be split into task types instead of checkpointed.
- example: repeated `task-start` for a running task is idempotent rather than creating another official output.

=== Rule: Memory Dreaming Skill Development - Temp Output Promotion
- condition: an agent completes a task.
- decision or calculation: the agent writes to script-assigned `tempOutputRef`; `task-complete` reads that exact path, validates schema, and promotes the official artifact.
- state transition: valid output writes formal artifacts and unlocks downstream tasks; invalid output leaves the task `RUNNING`.
- permission or data boundary: agents cannot submit arbitrary official artifact paths or inline large JSON.
- conflict or priority rule: `outputRef` is script-internal and not accepted from the caller.
- example: `task-complete --run-id X --task-id Y` reads the task's recorded temp file and refuses a malformed JSON body.

=== Rule: Memory Dreaming Skill Development - Script Only Memory Boundary
- condition: REM or Deep tasks run in this script-only slice.
- decision or calculation: support deterministic no-op or dry-run governance outputs without calling Memory MCP.
- state transition: REM and Deep can complete enough for status and finalization tests.
- permission or data boundary: no candidate or active memory record is written.
- conflict or priority rule: preserving task graph shape is more important than prompt quality in this slice.
- example: a Deep task can produce a no-op `deep-governance.json` when no sessions were imported.

=== Rule: Memory Dreaming Skill Development - Deterministic Finalization
- condition: `finalize-run` is invoked.
- decision or calculation: generate `dream-report.md` from formal artifacts only when `status.canFinalize=true`.
- state transition: the run becomes completed; repeated finalization returns the existing report.
- permission or data boundary: finalization does not call Memory MCP, rerun Deep, or accept an agent-written report body.
- conflict or priority rule: any `INIT`, `RUNNING`, or `FAILED` task blocks finalization.
- example: a failed segment summary prevents report generation until retried and completed.

=== Rule: Memory Dreaming Skill Development - Makefile First Verification
- condition: developers validate this script feature.
- decision or calculation: provide and use a root Makefile target for skill script tests before falling back to direct Python commands.
- state transition: automated validation can run from the repository root.
- permission or data boundary: tests must use fixtures and temp artifact roots, not live Codex sessions.
- conflict or priority rule: live smoke is optional and never replaces deterministic tests.
- example: `make skill-memory-dreaming-test` runs the Python unit tests for the skill scripts.

== Exceptions And Boundaries
=== Exception: Memory Dreaming Skill Development - Unsupported Provider
- scenario: a caller passes any provider other than `codex`.
- expected product behavior: command fails with a typed unsupported-provider error.
- user-visible message or fallback: tell the caller V1 supports only `codex`.
- data consistency expectation: no run or session artifacts are partially committed for that provider.
- retry, recovery, or compensation behavior: retry with `provider=codex`.
- logging/audit expectation: log provider name without session material.

=== Exception: Memory Dreaming Skill Development - Invalid Artifact Root
- scenario: the resolved artifact root, whether the default `.intentforge-dreaming/` or an explicit override, points to an unsafe location or is not writable.
- expected product behavior: command fails before importing sessions or changing task state.
- user-visible message or fallback: report path validation failure.
- data consistency expectation: no partial official artifacts are created.
- retry, recovery, or compensation behavior: fix the workspace `.intentforge-dreaming/` path for formal runs, or choose a writable isolated directory outside the repo for development smoke and tests.
- logging/audit expectation: include path metadata but not session text.

=== Exception: Memory Dreaming Skill Development - Malformed Codex Session
- scenario: a JSONL row cannot be parsed or normalized.
- expected product behavior: session receives `FAILED` in `session-index.json` when the failure is session-scoped; other sessions may continue.
- user-visible message or fallback: return import counts and failed session refs.
- data consistency expectation: failed snapshots are not treated as complete session material.
- retry, recovery, or compensation behavior: fix fixture/live input or ignore the failed session in a later run.
- logging/audit expectation: include source ref and parse reason without printing message bodies.

=== Exception: Memory Dreaming Skill Development - Invalid Task Transition
- scenario: caller completes an `INIT` task, retries a non-failed task, or finalizes while tasks remain open.
- expected product behavior: command rejects the transition and keeps current state unchanged.
- user-visible message or fallback: return the current state and allowed next actions.
- data consistency expectation: no official artifact is promoted on an invalid transition.
- retry, recovery, or compensation behavior: call the correct previous command such as `task-start` or `task-retry`.
- logging/audit expectation: include run id, task id, current state, and requested command.

=== Exception: Memory Dreaming Skill Development - Invalid Temp Output
- scenario: `tempOutputRef` is missing, unreadable, malformed, or fails schema validation.
- expected product behavior: `task-complete` fails validation, leaves the task `RUNNING`, and does not write the official artifact.
- user-visible message or fallback: return schema validation errors and the temp output ref.
- data consistency expectation: downstream tasks are not created from invalid output.
- retry, recovery, or compensation behavior: agent fixes the temp file and calls `task-complete` again.
- logging/audit expectation: include validation errors without copying full generated artifact content to stdout.

=== Exception: Memory Dreaming Skill Development - Finalization Blocked
- scenario: `status.canFinalize=false` because tasks are open, failed, Deep is incomplete, or the run is inconsistent.
- expected product behavior: `finalize-run` refuses to create or update `dream-report.md`.
- user-visible message or fallback: return blocker reasons from status.
- data consistency expectation: run remains unfinalized.
- retry, recovery, or compensation behavior: complete or retry blockers, then finalize again.
- logging/audit expectation: include run id and blocker codes.

=== Exception: Memory Dreaming Skill Development - Memory Integration Deferred
- scenario: a prompt or task expects Memory MCP reads or writes during this script-only slice.
- expected product behavior: script returns or documents a dry-run/no-op boundary instead of performing Memory Core operations.
- user-visible message or fallback: indicate Memory MCP integration is out of scope.
- data consistency expectation: no candidate or active memory records are written.
- retry, recovery, or compensation behavior: defer to the future prompt and Memory MCP integration feature.
- logging/audit expectation: record dry-run status in governance artifacts.

=== Exception: Memory Dreaming Skill Development - Runtime Loading Deferred
- scenario: a caller expects Tron backend or Codex runtime discovery to load this skill automatically.
- expected product behavior: no runtime loading happens in this slice.
- user-visible message or fallback: use the repo-local script path directly.
- data consistency expectation: backend skill registry state is unchanged.
- retry, recovery, or compensation behavior: future adoption feature installs or registers the validated skill.
- logging/audit expectation: not applicable for this slice.

== Shared Requirements
- EliteForge skill authoring contract: shared because every future `eliteforge-*` skill needs valid frontmatter, metadata version, optional environment declarations, output path rules, and `agents/openai.yaml`; consumers: Script Foundation, Prompt And Memory Integration, Backend And Runtime Adoption; decision: build first inside the script foundation.
- Dream artifact store contract: shared because scripts, prompts, tests, and future jobs must agree on `runs/` and `sessions/` files; consumers: Script Foundation, Prompt And Memory Integration, Backend And Runtime Adoption; decision: build first.
- Codex session normalization: shared because future prompts should consume provider-neutral snapshots instead of raw Codex JSONL; consumers: Script Foundation and Prompt And Memory Integration; decision: build first for Codex only.
- Task state machine contract: shared because prompt execution, output validation, retries, and final reports all depend on the same task lifecycle; consumers: Script Foundation and Prompt And Memory Integration; decision: build first.
- Memory MCP governance contract: shared because REM/Deep eventually need existing memory context; consumers: Prompt And Memory Integration and Backend And Runtime Adoption; decision: defer.
- Backend scheduling and registry contract: shared because future MemoryDreamJob needs safe runtime loading and a formal `.intentforge-dreaming/` artifact root policy; consumers: Backend And Runtime Adoption; decision: defer.

== Feature Inventory
1. Memory Dreaming Script Foundation: module Memory Dreaming Skill Development; source headings `Feature: Memory Dreaming Skill Development - Script Foundation`, `Page: Memory Dreaming CLI - Script Commands`, `Page: Memory Dreaming Artifact Workspace - Run And Session Files`, rules `Rule: Memory Dreaming Skill Development - Repo Local Skill Isolation` through `Rule: Memory Dreaming Skill Development - Makefile First Verification`, and exceptions `Exception: Memory Dreaming Skill Development - Unsupported Provider` through `Exception: Memory Dreaming Skill Development - Finalization Blocked`; goal deliver the tested repo-local CLI and artifact workflow; in scope skill scaffold, Codex import, artifact store, task state machine, temp output promotion, dry-run/no-op REM/Deep, report finalization, and tests; out of scope final prompts, Memory MCP, backend loading, frontend, and multi-provider support; delivery type current bounded feature; depends on approved brainstorming and existing Python/Codex local session format.
2. Memory Dreaming Prompt And MCP Integration: module Memory Dreaming Skill Development; source heading `Feature: Memory Dreaming Skill Development - Prompt And Memory Integration`; goal add production prompt contracts plus authorized Memory MCP-backed REM/Deep behavior; in scope semantic extraction and governance prompts; out of scope script foundation already completed; depends on Feature 1.
3. Memory Dreaming Backend Job Adoption: module Memory Dreaming Skill Development; source heading `Feature: Memory Dreaming Skill Development - Backend And Runtime Adoption`; goal let Tron schedule or trigger memory dreaming through backend services and safe skill loading; in scope runtime configuration and job orchestration; out of scope first script feature; depends on Feature 1 and Feature 2.
4. Memory Dreaming Review Surface: module future UI; source heading future review page; goal expose run reports, candidate memory points, and governance decisions to operators; in scope UI/API after backend adoption; out of scope script foundation; depends on Feature 3.
5. Memory Dreaming Multi-Provider Import: module future provider adapters; source heading future provider expansion; goal import Claude Code, OpenCode, or IntentForge sessions into the same snapshot model; in scope provider adapters and tests; out of scope first Codex-only script feature; depends on Feature 1.

== Dependency Map
#mermaid(
  "flowchart LR
    B[\"Approved Brainstorming: Memory Dreaming Script Boundary\"] --> F1[\"Feature 1: Memory Dreaming Script Foundation\"]
    F1 --> SR1[\"Shared Requirement: Dream Artifact Store Contract\"]
    F1 --> SR2[\"Shared Requirement: Task State Machine Contract\"]
    F1 --> F2[\"Feature 2: Prompt And MCP Integration\"]
    F2 --> F3[\"Feature 3: Backend Job Adoption\"]
    F3 --> F4[\"Feature 4: Review Surface\"]
    F1 --> F5[\"Feature 5: Multi-Provider Import\"]"
)

== Recommended Implementation Order
1. Memory Dreaming Script Foundation - this is the current bounded feature and must land before prompt quality or backend adoption.
2. Prompt And MCP Integration - this can depend on stable task detail, temp output, schema, and artifact promotion semantics.
3. Backend Job Adoption - this should wait until the skill is safe to load and the artifact path contract is stable.
4. Review Surface - this should consume backend job outputs and report artifacts rather than define the workflow.
5. Multi-Provider Import - this should reuse the Codex-proven snapshot model after V1 is stable.

== Acceptance
=== Acceptance: Memory Dreaming Skill Development - Script Foundation
- The repository contains `tron-skills/eliteforge-memory-dreaming` with valid `eliteforge-*` skill metadata, aligned `agents/openai.yaml`, and minimal prompt guidance only.
- The skill source is not placed under `.agents/skills` and is not automatically loaded by Tron backend or Codex runtime discovery.
- The CLI implements the V1 command set, defaults artifact reads and writes to `.intentforge-dreaming/`, and accepts `--artifact-root` for development and test isolation.
- `provider=codex` can scan active sessions, archived sessions, and session index metadata, and can normalize fixture sessions into snapshots without printing message bodies to stdout.
- `init-run` records scan decisions for import, reuse, resume, changed reimport, skip, and failed import cases.
- `snapshot.json` preserves source manifest, session, preprocess, events, segments, and stats, and tests cover normalized event category mapping.
- The artifact store preserves IntentForge-compatible `runs/` and `sessions/` layout under `.intentforge-dreaming/` and under any override root.
- The task lifecycle supports valid transitions, invalid transition rejection, retry, schema validation, temp output promotion, and downstream task creation.
- REM and Deep task positions can complete through deterministic dry-run or no-op outputs without Memory MCP calls.
- `status` blocks finalization until tasks and Deep are complete or no-op, and `finalize-run` creates an idempotent deterministic `dream-report.md`.
- Deterministic tests use fixtures and temporary artifact roots; development live `~/.codex` smoke writes outside the repository and leaves no committed runtime artifacts; formal runs use `.intentforge-dreaming/`.
- A Makefile-first validation target runs the skill script tests.

=== Acceptance: Memory Dreaming Skill Development - Future Prompt And Memory Integration
- Production task prompts are not required for the current script foundation and remain a later feature.
- Memory MCP reads and writes are not required for the current script foundation and remain a later feature.

== Risks And Open Questions
- Real Codex session JSONL can vary by CLI version; the importer must be tolerant and fixture coverage should include active and archived examples.
- Preserving full IntentForge artifact semantics while keeping V1 small requires minimal schemas that retain top-level compatibility without implementing final prompt quality.
- Adding a Makefile target touches the repository root, but it aligns script validation with the project rule to use Makefile first.
- No blocking product questions remain for the current Script Foundation feature.
