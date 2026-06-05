# Initial Agent Draft

status: approved_with_defaults
draft_source: docs/brainstorm/eliteforge-memory-dreaming/2026-05-28-22-43-10-prompt-quality-foundation.md
prd_source: docs/brainstorm/eliteforge-memory-dreaming/prompt-quality-prd.typ

This feature turns the existing memory dreaming script scaffold into a prompt-quality foundation that can be measured before the skill is considered usable.

# Feature Identity And Context

feature_id: eliteforge-memory-dreaming-prompt-quality-foundation
base_branch: feature/baiyihuan/memory/base
target_skill: tron-skills/eliteforge-memory-dreaming
dependency_features: eliteforge-memory-dreaming-script-foundation
dependency_contracts: AGENTS.md, eliteforge-codex-superpower, eliteforge-skill-spec, skill-creator, eliteforge-git-feature-oriented-lite-spec
related_skills: eliteforge-codex-superpower, eliteforge-skill-spec, skill-creator
related_project_rules: AGENTS.md, eliteforge-git-feature-oriented-lite-spec
prior_feature: eliteforge-memory-dreaming-script-foundation

The repository is developing an EliteForge skill that extracts durable memories from Codex conversations and governs those memories through REM and Deep dreaming stages. The current script foundation can import Codex sessions, create tasks, and accept task completion JSON, but the prompts and quality controls are not yet strong enough to trust real memory extraction.

# Requirement

Build a prompt-quality foundation for `eliteforge-memory-dreaming` that makes memory extraction structured, traceable, private by default, and objectively testable.

The implementation must:

- provide versioned prompt materials for each task type;
- expose prompt metadata through the script task contract;
- require provenance and evidence references in agent-produced JSON;
- generate REM work at memory-point granularity;
- evaluate quality with deterministic fixture manifests and semantic mutants;
- define a safe method for private live smoke testing on real Codex data;
- require a real Codex CLI session where the Agent uses the skill against private sampled Codex history before the skill is considered usable;
- make REM and Deep produce actionable dry-run governance decisions even when Memory MCP is unavailable;
- split long Codex sessions into bounded segment tasks so recall is not lost in one huge compression step;
- require claim-level traceability with event ids, ordinal ranges, source line ranges, evidence summaries, support types, claims, and content hashes;
- require Light memory points to include operational usage context, counterexamples, and scoring components so future Agents can apply them correctly;
- prevent real conversation material from being accidentally committed or leaked through normal test output.

# Product Source Slices

- `docs/brainstorm/eliteforge-memory-dreaming/2026-05-28-22-43-10-prompt-quality-foundation.md`
- `docs/brainstorm/eliteforge-memory-dreaming/prompt-quality-prd.typ`
- `/home/baiyihuan/code-ai/app-cisdigital-ai-intent-forge` memory dreaming design notes
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py`
- `tron-skills/eliteforge-memory-dreaming/tests/test_memory_dreaming.py`
- `tron-skills/eliteforge-memory-dreaming/SKILL.md`

# Scope Boundary

## Assumptions

- The current feature branch already contains the script foundation for `eliteforge-memory-dreaming`.
- V1 supports Codex first; other agent runtimes remain out of scope.
- Memory MCP writes are not performed by this script in V1.
- No Memory MCP is available in the current repository; REM and Deep must still perform dry-run governance analysis instead of becoming no-op placeholders.
- Real `~/.codex` smoke tests are local/private only and must not produce committed raw conversation data.
- Real Codex CLI execution on local real Codex conversation history is required for final quality acceptance; deterministic script tests only prove transport, validation, and regression behavior.
- The current preparation stage records the executable real-data evaluation protocol, but does not read real Codex history until the final run parameters are explicitly approved.
- The final prompt language can evolve, but the V1 prompt contracts must be testable and versioned.

## In Scope

- Prompt reference files for Light, Segment Summary, Session Summary, REM, and Deep tasks.
- Script changes that return prompt/version/hash/requirements in `task-detail`.
- V2 JSON schemas with provenance envelope and evidence references.
- Point-level REM task creation from Light memory points.
- Deterministic quality evaluator using semantic manifests, golden outputs, and negative mutants.
- Synthetic fixtures that cover durable preferences, decisions, corrections, scope boundaries, stale facts, tool noise, and evidence traceability.
- Private live-smoke helper or documented command path for sampled real Codex sessions.
- A mandatory private Codex CLI evaluation protocol: start a fresh Codex session, instruct the Agent to use the skill on sampled local real Codex history, and review the produced dreaming artifacts for memory quality.
- Script-side validation and formalization of agent-produced provenance, evidence references, and governance outputs.
- Git ignore and redaction controls for generated dreaming artifacts.

## Out Of Scope

- Production Memory MCP writes.
- UI changes.
- Backend API changes.
- Cross-agent runtime support beyond Codex.
- Committing real Codex session material, hashes, absolute private paths, or raw live smoke reports.
- Treating deterministic fixtures, schema validation, or helper-script success as proof that the skill is high quality.

## Constraints

- Use Makefile targets first for build, test, and cleanup.
- Preserve the uniform JSON response shape used by the script CLI.
- Keep generated runtime artifacts out of source control.
- Keep `SKILL.md` concise and move detailed prompts/schemas into references.
- Prefer deterministic regression validation for transport and privacy, but require real Codex CLI execution for skill-quality acceptance.

# Feature Model

Actors:

- Codex user: runs the skill and owns the local conversation corpus.
- Dreaming script: imports sessions, schedules tasks, validates task outputs, and creates artifacts.
- Codex Agent: uses the skill in a real Codex CLI session, reads task details, follows the prompt, and returns structured JSON.
- Quality evaluator: validates expected semantic memory behavior against fixtures.

Core objects:

- `dreamRun`: one import and processing run.
- `segment`: a bounded portion of a source Codex session.
- `task`: a Light, Segment Summary, Session Summary, REM, or Deep unit of work.
- `promptSpec`: versioned task prompt and output requirements.
- `evidenceRef`: structured pointer from derived memory back to source conversation material.
- `candidateSignal`: pre-Light durable-signal pool item preserved by Segment and Session tasks before final admission/rejection.
- `memoryPoint`: durable extracted memory candidate.
- `governanceDecision`: REM or Deep decision about a memory point.
- `semanticManifest`: fixture oracle describing expected and forbidden memory signals.

Flow:

1. `init-run` creates an artifact root and run metadata.
2. `import-codex` reads selected Codex sessions and writes redacted snapshot metadata plus bounded segment materials.
3. `task-detail` returns task payload, prompt metadata, task prompt, output schema expectations, and privacy-safe preview data.
4. In normal skill use, a real Codex CLI session is started and the Agent is instructed to use the skill to complete the dreaming tasks over the private prepared artifacts.
5. The Agent writes structured task outputs with provenance, claim-level evidence references, candidate signal coverage, extraction rationale, and governance decisions.
6. The script validates schema-level structure, formalizes accepted outputs, and verifies resolvable evidence references.
7. Accepted Light memory points create point-level REM tasks.
8. REM decisions create Deep tasks when governance rules require cross-point or cross-session handling.
9. Deep produces dry-run governance plans in V1; it never calls Memory MCP, but it must not be semantically empty when there is governance work to do.
10. The evaluator compares produced artifacts with semantic manifests and mutants.
11. Private Codex CLI evaluation runs prove that the skill can guide a real Agent to produce reviewable memory-analysis artifacts from historical sessions.

Invariants:

- Every committed memory point must be source-backed by at least one resolvable evidence reference.
- Whole-segment-only citations are insufficient; EvidenceRef must identify event ids, ordinal range, source line range, claim, support type, evidence summary, and content hash.
- Every Light point must state when to apply it, when not to apply it, usage guidance, counterexamples, quality warnings, and MemoryValueModel scoring components.
- Empty Light output is valid only when it includes a structured `noPointReason`.
- REM and Deep outputs are governance artifacts only in V1; they do not call Memory MCP, but they must still produce actionable dry-run decisions.
- Live smoke evidence must remain outside committed repository paths.
- Final completion cannot be claimed until the mandatory real-history Codex CLI evaluation runs and produces sanitized review evidence.

# Page And Module Surface

- `tron-skills/eliteforge-memory-dreaming/SKILL.md`: concise skill entrypoint and safety rules.
- `tron-skills/eliteforge-memory-dreaming/references/`: prompt, schema, and evaluation references.
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py`: CLI contract and task orchestration.
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_quality.py`: deterministic fixture evaluator.
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_live_smoke.py`: private live-smoke sampling helper or dry-run planner.
- `tron-skills/eliteforge-memory-dreaming/tests/`: script, schema, evaluator, privacy, and regression tests.
- `tron-skills/eliteforge-memory-dreaming/quality-fixtures/`: synthetic golden fixture bundles.
- `Makefile`: Makefile-first test and cleanup targets.
- `.gitignore`: generated dreaming artifact exclusions.

# Repository Findings

- The current script foundation already supports import, task listing, task detail, task completion, status, and finalize.
- Current task outputs are structurally useful but too loose for high-quality memory extraction.
- Current REM and Deep behavior is deterministic orchestration without enough semantic governance; this evolution must replace no-op behavior with explicit dry-run governance plans.
- Segment material can contain raw conversation text, so preview output and artifact roots need stronger privacy controls before real-data tests.
- `.intentforge-dreaming/` is not yet ignored and must be treated as generated private runtime output.
- Existing deterministic tests can be extended into a stronger evaluator instead of replaced wholesale.
- The original IntentForge design gives the dreaming pipeline shape, but this repository must exceed that baseline by proving real Codex CLI execution on private historical sessions.

# Open Decisions

- none
