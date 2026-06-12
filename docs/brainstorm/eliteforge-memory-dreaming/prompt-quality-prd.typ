= PRD: EliteForge Memory Dreaming Prompt Quality Foundation

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: prompt, schema, evaluator, provenance, and live-smoke quality foundation for the repo-local `eliteforge-memory-dreaming` skill.
- Problem: the script foundation can orchestrate tasks, but it cannot yet prove high-quality memory extraction, governance, traceability, or safe real Codex data validation.
- Target users: Tron skill developers, Codex agents executing dream tasks, future memory-governance reviewers, and maintainers of MemoryDreamJob adoption.
- Outcome: task prompts, v2 schemas, manifest-driven quality fixtures, provenance enforcement, real Codex live-smoke method, and security hardening make memory dreaming quality auditable.

== Background
- Why now: the existing script foundation is stable enough to execute runs, but production prompt work would be unsafe without measurable quality gates.
- Current pain: schema-valid output can still be semantically wrong, polluted, untraceable, or unsafe to validate on real `~/.codex` data.
- Business context: memory dreaming will influence future long-term memory. Incorrect extraction can pollute user, agent, and space memory, so quality and provenance are first-class requirements.

== Goals
- Provide concise skill guidance plus detailed task prompts for all five dreaming task types.
- Make `task-detail` return prompt metadata, prompt text, prompt hash, and structured task requirements.
- Upgrade task outputs to v2 schemas that expose evidence, provenance, quality scores, empty Light results, and governance decisions.
- Change REM governance from session-level to point-level tasks.
- Add deterministic semantic fixtures, golden artifacts, schema-valid mutants, and a manifest-driven evaluator.
- Define a real Codex live-smoke method that uses sampled local data without committing private artifacts.
- Add safety hardening for runtime artifact ignore rules, live preview leakage, temp file permissions, and cleanup.

== Scope
- In scope: prompt references, `task-detail` prompt contract, v2 schemas, provenance envelope, shared `EvidenceRef`, point-level REM task generation, semantic manifest evaluator, fixture bundles, mutants, real Codex live-smoke helper/reporting method, `.gitignore` protection, sensitive artifact handling, and Makefile-first validation.
- Out of scope: real Memory MCP calls, Memory Core writes, backend scheduling, frontend UI, multi-provider import, fully automatic CI over private `~/.codex`, and committing live data artifacts.

== Feature Requirements
=== Module: Memory Dreaming Prompt Quality
==== Feature: Memory Dreaming Prompt Quality - Prompt Contract
- name: task prompt contract exposed through the skill and CLI.
- description: provide executable task prompts and quality requirements for `SEGMENT_SUMMARY`, `SESSION_SUMMARY`, `LIGHT_MEMORY_POINTS`, `REM_GOVERN_MEMORY_POINT`, and `DEEP_MEMORY_GOVERNANCE`.
- user or role: Codex agent executing dream tasks.
- entry point: `task-detail`.
- trigger: an agent requests details for an `INIT` or `RUNNING` task.
- preconditions: prompt reference files exist under the skill `references/` directory.
- main flow: load shared and task-specific prompt files, build prompt metadata, return prompt version/hash, task prompt, requirements, schema, input refs, preview policy, and temp output ref.
- inputs and outputs: task registry and prompt files in; `taskPrompt`, `promptVersion`, `promptHash`, and `requirements` out.
- permissions or visibility: prompts may mention future Memory MCP semantics only as design context; current executable requirements must keep REM/Deep script-only.
- related page headings: `Page: Memory Dreaming CLI - Task Detail Prompt Contract`.
- related business rule headings: `Rule: Memory Dreaming Prompt Quality - Prompt Version And Hash`.
- related exception headings: `Exception: Memory Dreaming Prompt Quality - Missing Prompt Reference`.

==== Feature: Memory Dreaming Prompt Quality - V2 Output Schemas And Provenance
- name: structured, auditable task artifact schemas.
- description: upgrade task output schemas so semantic claims, evidence, prompt metadata, and governance decisions are machine-checkable.
- user or role: dreaming script, quality evaluator, future memory-governance reviewer.
- entry point: `task-complete`.
- trigger: an agent writes task output to `tempOutputRef` and completes the task.
- preconditions: task is `RUNNING`; temp output exists; schema version matches task type.
- main flow: validate provenance envelope, validate task-specific fields, validate shared `EvidenceRef` structure, promote output, and create downstream tasks.
- inputs and outputs: v2 task output in; formal task artifact and downstream task records out.
- permissions or visibility: no Memory MCP calls in this feature.
- related page headings: `Page: Memory Dreaming Artifact Workspace - V2 Task Artifacts`.
- related business rule headings: `Rule: Memory Dreaming Prompt Quality - EvidenceRef Required`, `Rule: Memory Dreaming Prompt Quality - Empty Light Result`.
- related exception headings: `Exception: Memory Dreaming Prompt Quality - Invalid Provenance`, `Exception: Memory Dreaming Prompt Quality - Unresolvable EvidenceRef`.

==== Feature: Memory Dreaming Prompt Quality - Semantic Quality Evaluator
- name: manifest-driven evaluator for deterministic quality fixtures.
- description: compare artifacts against semantic manifests, golden outputs, and schema-valid mutants so semantically wrong outputs fail even when JSON is valid.
- user or role: skill developer and reviewer.
- entry point: quality evaluator script and Makefile target.
- trigger: deterministic tests or manual verification.
- preconditions: fixture bundle includes `semantic-manifest.json`, fixture Codex data, mock memory state, golden artifacts, and mutants.
- main flow: schema-validate artifacts, normalize semantic rows, resolve evidence refs, compare expected points, reject forbidden signals, verify REM/Deep decisions, and emit deterministic quality reports.
- inputs and outputs: fixture bundle and artifacts in; `quality-report.json` and optional Markdown summary out.
- permissions or visibility: evaluator must not call a model or inspect live private data during deterministic tests.
- related page headings: `Page: Memory Dreaming Quality Evaluator - Fixture Evaluation`.
- related business rule headings: `Rule: Memory Dreaming Prompt Quality - Manifest Oracle`, `Rule: Memory Dreaming Prompt Quality - Hard Fail Safety Gates`.
- related exception headings: `Exception: Memory Dreaming Prompt Quality - Schema Valid But Semantically Wrong`.

==== Feature: Memory Dreaming Prompt Quality - Real Codex Live Smoke
- name: private local smoke for real Codex history.
- description: validate prompt stability on sampled real local Codex data while preventing private artifact commits.
- user or role: skill developer.
- entry point: live smoke helper or documented command sequence.
- trigger: developer explicitly requests live smoke.
- preconditions: local `~/.codex` exists; developer chooses or generates sample; explicit repo-external artifact root is provided.
- main flow: build symlink sample Codex home, run locked sample and fresh holdout sample, execute dream task cycle, run evaluator, repeat stable samples, and write private local evidence.
- inputs and outputs: sampled local JSONL files in; private local artifacts and aggregate committed summary out.
- permissions or visibility: live smoke is supplemental evidence and must not become a default CI gate.
- related page headings: `Page: Memory Dreaming Live Smoke - Real Codex Sample`.
- related business rule headings: `Rule: Memory Dreaming Prompt Quality - Live Smoke Privacy`, `Rule: Memory Dreaming Prompt Quality - Stability Threshold`.
- related exception headings: `Exception: Memory Dreaming Prompt Quality - Live Artifact In Repository`.

==== Feature: Memory Dreaming Prompt Quality - Security And Retention
- name: data safety hardening for real-session work materials.
- description: prevent accidental commits, stdout leaks, broad temp permissions, and stale real-data artifacts.
- user or role: skill developer and repository maintainer.
- entry point: CLI commands, temp output handling, `.gitignore`, cleanup command.
- trigger: any dreaming command that reads or writes artifacts.
- preconditions: repository may contain no live artifacts.
- main flow: ignore runtime roots, use private temp permissions, redact or suppress live previews, avoid public absolute paths when possible, delete temp files after promotion, and provide cleanup.
- inputs and outputs: runtime artifacts and temp outputs in; protected workspace and cleanup status out.
- permissions or visibility: no private paths, previews, message text, hashes, or distilled facts in committed reports.
- related page headings: `Page: Memory Dreaming CLI - Safe Runtime Execution`.
- related business rule headings: `Rule: Memory Dreaming Prompt Quality - No Live Artifact Commit`, `Rule: Memory Dreaming Prompt Quality - Sensitive Preview Policy`.
- related exception headings: `Exception: Memory Dreaming Prompt Quality - Preview Redacted`, `Exception: Memory Dreaming Prompt Quality - Temp Output Cleanup Failure`.

== Page Prototypes And Interactions
=== Page: Memory Dreaming CLI - Task Detail Prompt Contract
- entry: `python3 tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py task-detail --run-id <id> --task-id <id>`.
- layout: JSON CLI payload for agents.
- data display: input refs, schema, temp output ref, prompt version, prompt hash, prompt text, requirements, and preview policy.
- actions: request task details, inspect prompt requirements, write task output to temp path.
- states: prompt available, prompt missing, preview available, preview redacted.

==== Interaction: Memory Dreaming CLI - Complete Prompted Task
- trigger control: `task-detail`, `task-start`, write `tempOutputRef`, then `task-complete`.
- preconditions: prompt reference files load successfully and schema version matches the task.
- button or control behavior: not applicable.
- form rules: output must include provenance, evidence refs, and task-specific v2 fields.
- confirmation behavior: `task-complete` validates before promotion.
- success feedback: formal artifact is written and downstream tasks are created.
- failure feedback: invalid schema, provenance, or evidence leaves the task running with a typed error.
- navigation or refresh behavior: `task-list` and `status` reflect resulting task graph.

=== Page: Memory Dreaming Quality Evaluator - Fixture Evaluation
- entry: quality evaluator script through Makefile.
- layout: CLI report.
- data display: hard-fail status, precision, recall, pollution, evidence integrity, scope/type accuracy, governance accuracy, idempotency, and stability metrics.
- actions: evaluate golden outputs, evaluate mutants, emit report.
- states: pass, hard fail, metric fail, fixture invalid.

=== Page: Memory Dreaming Live Smoke - Real Codex Sample
- entry: explicit live-smoke command or documented command sequence using `--codex-home <sample-home>` and `--artifact-root <repo-outside-root>`.
- layout: private local artifact workspace and redacted summary report.
- data display: bucket counts, aggregate metrics, stability summary, cleanup status.
- actions: generate sample home, run smoke, evaluate, cleanup.
- states: locked sample run, holdout run, repeated run, redacted failure, cleaned.

== Business Rules
=== Rule: Memory Dreaming Prompt Quality - Prompt Version And Hash
- condition: `task-detail` returns a task prompt.
- decision or calculation: include prompt version and SHA-256 hash for the exact prompt bundle used by the task.
- state transition: promoted task output records matching prompt metadata.
- permission or data boundary: prompt metadata is safe to commit.
- conflict or priority rule: stale output with mismatched prompt metadata fails validation.
- example: `LIGHT_MEMORY_POINTS` output records `promptHash` matching the returned prompt bundle.

=== Rule: Memory Dreaming Prompt Quality - EvidenceRef Required
- condition: a task output contains semantic claims or governance decisions.
- decision or calculation: each accepted point and decision must contain resolvable `EvidenceRef` entries.
- state transition: unresolvable evidence blocks task completion or fails evaluator.
- permission or data boundary: evidence refs point to artifacts and hashes, not copied private text.
- conflict or priority rule: no evidence means no accepted memory.
- example: a user preference point references `sessionKey`, `segmentId`, `eventIds`, `sourceRole=USER`, and `sourceHash`.

=== Rule: Memory Dreaming Prompt Quality - Empty Light Result
- condition: a session has no durable memory signal.
- decision or calculation: `LIGHT_MEMORY_POINTS` may output `points=[]` with `noPointReason` and optional `rejectedSignals`.
- state transition: no point-level REM tasks are created for that session.
- permission or data boundary: empty output is valid and must not force garbage memory generation.
- conflict or priority rule: quality beats volume.
- example: a session that only contains one-off progress notes produces no points.

=== Rule: Memory Dreaming Prompt Quality - Manifest Oracle
- condition: deterministic quality tests run.
- decision or calculation: compare artifacts against `semantic-manifest.json` using canonical keys and explicit aliases.
- state transition: schema-valid but semantically wrong artifacts fail.
- permission or data boundary: fixture data is synthetic and commit-safe.
- conflict or priority rule: hard-fail safety gates override numeric scores.
- example: a mutant that accepts tool output as a user fact fails even if recall is high.

=== Rule: Memory Dreaming Prompt Quality - Live Smoke Privacy
- condition: real local Codex data is used.
- decision or calculation: use a sampled symlink Codex home and explicit repo-external artifact root.
- state transition: private local evidence may be generated; committed evidence contains aggregates only.
- permission or data boundary: no source path, session id, source hash, preview, message text, or private fact is committed.
- conflict or priority rule: privacy failure invalidates live smoke evidence.
- example: a committed report states bucket counts and metrics, not sampled file names.

=== Rule: Memory Dreaming Prompt Quality - No Live Artifact Commit
- condition: git status or validation sees runtime memory dreaming artifacts.
- decision or calculation: fail validation if `.intentforge-dreaming/`, live `runs/`, live `sessions/`, or temp output files are inside the repository.
- state transition: developer must cleanup before completion.
- permission or data boundary: private artifacts remain local only.
- conflict or priority rule: accidental artifact visibility blocks delivery.
- example: `.intentforge-dreaming/sessions/.../segments/*.material.md` must never be committed.

=== Rule: Memory Dreaming Prompt Quality - Stability Threshold
- condition: live smoke is run on locked or holdout samples.
- decision or calculation: compare canonical memory keys across repeated runs.
- state transition: unstable prompt output blocks quality approval.
- permission or data boundary: only aggregate stability metrics are reported.
- conflict or priority rule: stable safety behavior matters more than extracting more memories.
- example: repeated runs should not flip the same point between `NO_WRITE`, candidate creation, and conflict review without explanation.

== Exceptions And Boundaries
=== Exception: Memory Dreaming Prompt Quality - Missing Prompt Reference
- scenario: `task-detail` cannot load shared or task-specific prompt reference files.
- expected product behavior: return typed error and do not assign a misleading generic prompt.
- user-visible message or fallback: identify missing prompt file.
- data consistency expectation: task state does not change.
- retry, recovery, or compensation behavior: restore prompt file and retry.
- logging/audit expectation: report prompt file path and task type.

=== Exception: Memory Dreaming Prompt Quality - Invalid Provenance
- scenario: task output lacks provenance envelope or prompt hash mismatch.
- expected product behavior: reject completion.
- user-visible message or fallback: explain missing or mismatched provenance field.
- data consistency expectation: formal artifact is not promoted.
- retry, recovery, or compensation behavior: agent rewrites temp output.
- logging/audit expectation: validation error is returned without private text.

=== Exception: Memory Dreaming Prompt Quality - Unresolvable EvidenceRef
- scenario: an evidence ref cannot be resolved to a session, segment, event, or source hash.
- expected product behavior: reject completion or fail evaluator.
- user-visible message or fallback: list unresolved evidence keys.
- data consistency expectation: no accepted memory without evidence.
- retry, recovery, or compensation behavior: correct evidence refs or reject the point.
- logging/audit expectation: log ref identifiers only.

=== Exception: Memory Dreaming Prompt Quality - Schema Valid But Semantically Wrong
- scenario: mutant artifact is valid JSON but accepts forbidden signal, wrong scope, wrong type, missing evidence, or unsafe Deep action.
- expected product behavior: evaluator hard-fails.
- user-visible message or fallback: report semantic violation category.
- data consistency expectation: metrics record failure.
- retry, recovery, or compensation behavior: fix prompt, schema, or evaluator as appropriate.
- logging/audit expectation: synthetic fixture details only.

=== Exception: Memory Dreaming Prompt Quality - Live Artifact In Repository
- scenario: live smoke writes artifacts under the repository.
- expected product behavior: validation fails.
- user-visible message or fallback: instruct cleanup and rerun with repo-external artifact root.
- data consistency expectation: no live artifacts are committed.
- retry, recovery, or compensation behavior: delete artifacts, verify git status, rerun safely.
- logging/audit expectation: no private content in logs.

=== Exception: Memory Dreaming Prompt Quality - Preview Redacted
- scenario: task detail is requested for live data with preview disabled or redacted.
- expected product behavior: return prompt and refs without private preview text.
- user-visible message or fallback: indicate preview was redacted by safety policy.
- data consistency expectation: task can still proceed by reading local refs when authorized.
- retry, recovery, or compensation behavior: use explicit local inspection only outside committed reports.
- logging/audit expectation: no message text in stdout.

=== Exception: Memory Dreaming Prompt Quality - Temp Output Cleanup Failure
- scenario: promoted temp output cannot be removed or temp directory permissions are unsafe.
- expected product behavior: report cleanup warning and provide purge command.
- user-visible message or fallback: identify temp root without printing private content.
- data consistency expectation: formal artifact remains promoted.
- retry, recovery, or compensation behavior: run cleanup command or Makefile target.
- logging/audit expectation: no temp file content in logs.

== Shared Requirements
- Script foundation: existing Codex import, task lifecycle, temp output promotion, status, and finalization; consumers: all prompt quality features; decision: depends on delivered script foundation.
- Provenance model: shared `EvidenceRef` and provenance envelope; consumers: schemas, evaluator, live smoke, future Memory MCP integration; decision: build first.
- Security guardrails: gitignore, preview policy, temp permissions, cleanup; consumers: live smoke and any future runtime adoption; decision: build first.
- Quality evaluator: shared manifest oracle and metric reports; consumers: deterministic fixtures and live smoke; decision: build before prompt approval.

== Feature Inventory
1. Prompt Contract: module Memory Dreaming Prompt Quality; source heading `Feature: Memory Dreaming Prompt Quality - Prompt Contract`; pages `Page: Memory Dreaming CLI - Task Detail Prompt Contract`; rules `Prompt Version And Hash`; exceptions `Missing Prompt Reference`; goal expose executable prompts; depends on script foundation.
2. V2 Output Schemas And Provenance: source heading `Feature: Memory Dreaming Prompt Quality - V2 Output Schemas And Provenance`; pages `Artifact Workspace`; rules `EvidenceRef Required`, `Empty Light Result`; exceptions `Invalid Provenance`, `Unresolvable EvidenceRef`; goal make outputs auditable; depends on Prompt Contract and Provenance model.
3. Semantic Quality Evaluator: source heading `Feature: Memory Dreaming Prompt Quality - Semantic Quality Evaluator`; pages `Quality Evaluator`; rules `Manifest Oracle`, `Hard Fail Safety Gates`; exceptions `Schema Valid But Semantically Wrong`; goal deterministic quality proof; depends on V2 schemas.
4. Real Codex Live Smoke: source heading `Feature: Memory Dreaming Prompt Quality - Real Codex Live Smoke`; pages `Live Smoke`; rules `Live Smoke Privacy`, `Stability Threshold`; exceptions `Live Artifact In Repository`; goal private real-data validation; depends on evaluator and security guardrails.
5. Security And Retention: source heading `Feature: Memory Dreaming Prompt Quality - Security And Retention`; pages `Safe Runtime Execution`; rules `No Live Artifact Commit`, `Sensitive Preview Policy`; exceptions `Preview Redacted`, `Temp Output Cleanup Failure`; goal prevent private data leakage; depends on script foundation.

== Dependency Map
#mermaid(
  "flowchart LR
    A[\"Script Foundation\"] --> B[\"Security And Retention\"]
    A --> C[\"Prompt Contract\"]
    B --> D[\"V2 Schemas And Provenance\"]
    C --> D
    D --> E[\"Semantic Quality Evaluator\"]
    E --> F[\"Real Codex Live Smoke\"]"
)

== Recommended Implementation Order
1. Security And Retention - real-data smoke is unsafe until runtime roots, previews, temp files, and cleanup are guarded.
2. Prompt Contract - agents need stable task prompts and prompt hashes before producing v2 artifacts.
3. V2 Output Schemas And Provenance - evaluator and live smoke require inspectable evidence and provenance.
4. Semantic Quality Evaluator - deterministic fixtures and mutants prove quality gates before real-data smoke.
5. Real Codex Live Smoke - supplemental stability validation after deterministic gates pass.

== Acceptance
=== Acceptance: Memory Dreaming Prompt Quality - Prompt Contract
- `task-detail` returns prompt version, prompt hash, task prompt, and structured requirements for every task type, and fails clearly when prompt references are missing.

=== Acceptance: Memory Dreaming Prompt Quality - V2 Schemas And Provenance
- task completion enforces v2 schemas, provenance envelopes, shared `EvidenceRef`, empty Light results, and point-level REM task creation.

=== Acceptance: Memory Dreaming Prompt Quality - Semantic Evaluator
- deterministic fixture tests prove golden artifacts pass and schema-valid semantic mutants fail by hard gate or metric threshold.

=== Acceptance: Memory Dreaming Prompt Quality - Real Codex Live Smoke
- documented or scripted live smoke uses sampled symlink Codex homes, repo-external artifact roots, repeated locked/holdout runs, private local evidence, and committed aggregate-only results.

=== Acceptance: Memory Dreaming Prompt Quality - Security And Retention
- runtime artifacts and temp outputs are protected from commit, live previews are redacted or opt-in, temp files use private permissions and cleanup, and validation catches live artifacts inside the repo.

== Risks And Open Questions
- REM per point changes task graph shape; keep compatibility only where it does not weaken quality.
- Real data smoke cannot be reproduced by CI; deterministic fixtures remain the primary gate.
- Redaction can remove useful context; prompts must prefer evidence refs and hashes when text cannot be safely shown.
- Future Memory MCP integration will need a separate feature to convert dry-run governance decisions into authorized writes.
