# Test Plan Decision

status: approved_with_defaults

The feature is validated through deterministic script tests, prompt/schema validation, fixture-based semantic evaluation, a private live-smoke dry-run path, and a mandatory private Codex CLI evaluation session on local real Codex conversation history. Automated tests must not depend on real `~/.codex` data, but final skill-quality acceptance remains blocked until the real-history Codex CLI evaluation runs and is reviewed.

AC-MDPQ-08 through AC-MDPQ-12 are driven by the atomic source-analysis matrices and `source-coverage-audit.md` in this spec directory. Matrix row ids such as `FLOW-*`, `QUALITY-*`, `GOV-*`, `LTM-*`, and `GAP-*` are local planning references only; `AC-MDPQ-*` remains the acceptance source of truth.

# Matrix Coverage

- AC-MDPQ-08 | matrix_refs: `requirements-flow-matrix.md` FLOW-001..FLOW-055; `requirements-quality-matrix.md` QUALITY-071, QUALITY-085; `source-coverage-audit.md` FLOW/QUALITY task-boundary rows; `current-gap-matrix.md` GAP-001..GAP-002 | proof focus: real Codex Agent workflow, script-governed task sequence, task state safety, recovery, original source traceability, and no automation runner replacing the Agent.

- AC-MDPQ-09 | matrix_refs: `requirements-quality-matrix.md` QUALITY-001..QUALITY-085; `requirements-long-term-memory-matrix.md` LTM-001..LTM-078; `source-coverage-audit.md` QUALITY/LTM rows; `current-gap-matrix.md` GAP-003..GAP-007 | proof focus: high-quality memory extraction, evidence source discipline, Segment/Session quality, four-axis positioning, quality scoring, hard exclusions, source examples as fixtures, and pollution prevention.

- AC-MDPQ-10 | matrix_refs: `requirements-governance-matrix.md` GOV-001..GOV-056; `requirements-quality-matrix.md` QUALITY-050..QUALITY-085; `requirements-flow-matrix.md` FLOW-020..FLOW-055; `requirements-long-term-memory-matrix.md` LTM-014..LTM-078; `source-coverage-audit.md` GOV/Deep rows; `current-gap-matrix.md` GAP-008..GAP-011 | proof focus: no-MCP dry-run governance, proposed memory operations, authorization evidence, conflict/stale/merge/promotion handling, MCP boundary enforcement, and rejection of false no-op.

- AC-MDPQ-11 | matrix_refs: `requirements-flow-matrix.md` FLOW-001..FLOW-055; `requirements-quality-matrix.md` QUALITY-071..QUALITY-085; `requirements-long-term-memory-matrix.md` LTM-001..LTM-078; `source-coverage-audit.md` private-eval relevant rows; `current-gap-matrix.md` GAP-012 | proof focus: private real Codex CLI session uses the skill against sampled history and completes Segment, Session, Light, REM, Deep, report artifacts, and original-source quality checks.

- AC-MDPQ-12 | matrix_refs: `requirements-flow-matrix.md` FLOW-006, FLOW-017, FLOW-021, FLOW-022, FLOW-044..FLOW-055; `requirements-quality-matrix.md` QUALITY-012, QUALITY-017, QUALITY-048, QUALITY-059, QUALITY-071..QUALITY-085; `requirements-governance-matrix.md` GOV-049..GOV-056; `requirements-long-term-memory-matrix.md` LTM-002, LTM-010, LTM-024..LTM-026, LTM-031..LTM-078; `source-coverage-audit.md` privacy/private-eval rows; `current-gap-matrix.md` GAP-013..GAP-014 | proof focus: no raw session text, private paths, session ids, source hashes, sensitive governance traces, or unsanitized private memory subjects in committed evidence.

- AC-MDPQ-13 | matrix_refs: `current-gap-matrix.md` GAP-003, GAP-012; `requirements-flow-matrix.md` FLOW-012..FLOW-025; `requirements-quality-matrix.md` QUALITY-019..QUALITY-035; original-design compression and task-boundary rows in `source-coverage-audit.md` | proof focus: bounded message-window segmentation, multiple Segment tasks for long sessions, candidateSignals, discarded findings, correction timeline, and coverageStats before Light compression.

- AC-MDPQ-14 | matrix_refs: `requirements-quality-matrix.md` QUALITY-003..QUALITY-018, QUALITY-036..QUALITY-041; `requirements-long-term-memory-matrix.md` LTM-063..LTM-078; `source-coverage-audit.md` evidence/locality rows | proof focus: claim-level EvidenceRef fields, resolvable event ids, ordinal ranges, source line ranges, evidence summaries, support types, claims, content hashes, and rejection of whole-segment-only evidence.

- AC-MDPQ-15 | matrix_refs: `requirements-quality-matrix.md` QUALITY-001..QUALITY-085; `requirements-long-term-memory-matrix.md` LTM-001..LTM-078; `current-gap-matrix.md` GAP-004..GAP-007 | proof focus: richer memory point shape with memoryType, matching category, typeSpecific, applyWhen, doNotApplyWhen, usageGuidance, counterExamples, qualityWarnings, MemoryValueModel scoring components, target scope, subject, and purpose.

- AC-MDPQ-16 | matrix_refs: `requirements-quality-matrix.md` QUALITY-050..QUALITY-085; `requirements-long-term-memory-matrix.md` LTM-063..LTM-078; evaluator fixture rows in `source-coverage-audit.md` | proof focus: deterministic evaluator and fixture mutants catch thin content, missing usage context, missing memoryType/typeSpecific, mismatched type/category, missing evidence locality, source pollution, stale correction, wrong scope, and no-memory false positives.

- AC-MDPQ-17 | matrix_refs: `current-gap-matrix.md` GAP-012..GAP-014; `requirements-quality-matrix.md` QUALITY-071..QUALITY-085; `requirements-governance-matrix.md` GOV-049..GOV-056; `private-evaluation.md` review rubric | proof focus: private real-history evaluation rubric includes content richness, traceability coverage, recall/compression pressure, no-op/no-memory reasoning, privacy redaction, and aggregate-only reporting.

# Real Data Quality Gate

- The real Codex session gate is mandatory for final skill completion; synthetic fixtures, schema tests, helper-script dry runs, and evaluator passes cannot replace it.
- The gate must use local real Codex conversation data discovered from `$CODEX_HOME`, `~/.codex`, or another user-approved Codex history root.
- The gate is executed only in the final validation stage after the user approves the exact prompt, sampling scope, artifact root, redaction policy, and quality rubric. The current preparation stage records the testable protocol and must not read real Codex history.
- The execution must be a real Codex CLI session where the Agent is instructed to use `tron-skills/eliteforge-memory-dreaming`; helper scripts may prepare inputs, enforce task order, validate JSON, sanitize evidence, and summarize metrics, but must not replace the Agent's semantic dreaming work.
- Passing criteria must judge extraction quality, governance reasoning, memory pollution prevention, source traceability, scope placement, privacy redaction, and reviewer conclusion against AC-MDPQ-11, AC-MDPQ-12, LTM-078, and the source-backed matrix rows.
- Private artifacts stay outside the repository. Committed evidence may include only aggregate quality scores, failure categories, redaction status, matrix failure buckets, and reviewer conclusion.

# Proof Rows

- AC-MDPQ-01 | layer: integration | case: task-detail returns prompt metadata for all task types | red: current detail lacks versioned prompt contract | green: promptVersion, promptHash, taskPrompt, outputSchema, requirements present | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-02 | layer: integration | case: V2 task completion validates provenance and creates point REM | red: malformed evidence and missing noPointReason are accepted | green: invalid artifacts fail and valid points schedule REM per point | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-03 | layer: security | case: previews and generated roots avoid real-data leaks | red: raw material appears in default task detail or Git status | green: preview is redacted by default and generated roots are ignored | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-04 | layer: regression | case: semantic evaluator scores golden fixtures and rejects mutants | red: evaluator accepts polluted, missing-evidence, or wrong-scope outputs | green: golden passes and each mutant fails for the expected reason | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY

- AC-MDPQ-05 | layer: manual | case: private live smoke plan uses sampled Codex homes | red: command would scan all sessions or write committed private output | green: dry-run emits a sampled local plan and aggregate-only report shape | command_ref: CMD-LIVE-SMOKE-DRY

- AC-MDPQ-06 | layer: validation | case: skill metadata, references, docs, and Makefile targets validate | red: skill references are missing or feature docs fail validator | green: skill validation and feature docs validation pass | command_ref: CMD-SKILL-VALIDATE

- AC-MDPQ-07 | layer: security | case: REM and Deep reject Memory MCP calls | red: governance output can record nonzero Memory MCP writes | green: task-complete rejects nonzero calls and final report records zero writes | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-08 | layer: skill | case: SKILL.md directs real Codex Agent usage | red: skill only tells users to run helper scripts and does not define agent reasoning flow | green: skill entrypoint defines safe real-data dreaming workflow, task sequence, quality gates, escalation boundaries, and matrix coverage for FLOW/GAP workflow rows | command_ref: CMD-SKILL-VALIDATE

- AC-MDPQ-09 | layer: prompt | case: prompts enforce high-quality memory extraction | red: prompts allow unsourced facts, noisy tool output, stale facts, broad personal profiling, or weak cards that violate QUALITY/LTM rows | green: prompts and fixtures require durable value gates, evidence refs, correction handling, source-role boundaries, no-point reasons, quality scoring, and four-axis placement | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY

- AC-MDPQ-10 | layer: governance | case: REM and Deep perform dry-run governance without Memory MCP | red: REM or Deep returns deterministic no-op while unresolved merge, conflict, stale, promotion, reject, authorization, or structure work exists | green: outputs record no-MCP decisions, reasons, proposed memory operations, read/write intents, required authorization flags, and escalation coverage for GOV/QUALITY/LTM rows | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-11 | layer: manual-private | case: real Codex CLI session uses the skill on sampled local history | red: evaluation only proves scripts, synthetic fixtures, or dry-run planning | green: mandatory private Codex CLI session instructs the Agent to use the skill and produces reviewable Segment, Session, Light, REM, Deep, and report artifacts scored against FLOW/LTM quality rows | command_ref: CMD-CODEX-CLI-SKILL-EVAL

- AC-MDPQ-12 | layer: security | case: real Codex CLI evaluation evidence is sanitized | red: committed evidence includes raw session text, private paths, session ids, source hashes, raw governance excerpts, or private memory subjects | green: committed evidence records only aggregate quality scores, failure categories, redaction status, matrix failure buckets, and reviewer conclusion | command_ref: CMD-CODEX-CLI-SKILL-EVAL

- AC-MDPQ-13 | layer: integration | case: long sessions split before Segment analysis | red: one huge segment compresses thousands of messages into a few notes and Light loses recall | green: `init-run` records bounded-message-window segmentation, segment metadata, and one Segment task per segment; Segment/Session outputs require candidateSignals and coverageStats | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-14 | layer: integration | case: EvidenceRef is claim-level and resolvable | red: whole segment/file evidence without event ids, ordinal range, source line range, summary, support type, claim, or content hash is accepted | green: task completion and evaluator reject missing locality and accept claim-level EvidenceRef values | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST

- AC-MDPQ-15 | layer: prompt | case: Light points are usable without rereading sessions | red: a one-sentence normalizedText plus generic governanceHints is accepted | green: Light schema, prompts, fixtures, and validator require memoryType, matching category, typeSpecific, applyWhen, doNotApplyWhen, usageGuidance, counterExamples, qualityWarnings, scoring components, target scope, subject, and purpose | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY

- AC-MDPQ-16 | layer: regression | case: evaluator rejects quality regressions beyond simple schema shape | red: evaluator accepts thin memory content, missing usage fields, missing source line range, source pollution, stale facts, wrong scope, type mismatch, missing typeSpecific, or no-memory false positives | green: quality evaluator passes golden fixtures and rejects all expected mutants including missing applyWhen, missing sourceLineRange, missing memoryType, memoryType/category mismatch, and missing typeSpecific | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY

- AC-MDPQ-17 | layer: manual-private | case: real Codex CLI review judges memory quality, not only completion | red: private eval can pass with task counts and privacy checks while extracted memories remain thin or untraceable | green: private-evaluation rubric and report fields require content richness, traceability coverage, recall/compression pressure, no-op/no-memory reasoning, privacy redaction, and sanitized aggregate review | command_ref: CMD-CODEX-CLI-SKILL-EVAL

# Command Index

- CMD-SKILL-MEMORY-DREAMING-TEST | command: make skill-memory-dreaming-test | cost: medium | when: before commit and after script changes | rerun: after prompt/schema/script/test edits | expect: all memory dreaming unit and integration tests pass

- CMD-SKILL-MEMORY-DREAMING-QUALITY | command: make skill-memory-dreaming-quality-test | cost: medium | when: before commit and after prompt/evaluator edits | rerun: after fixture, evaluator, or schema edits | expect: golden fixtures pass and semantic mutants fail

- CMD-SKILL-VALIDATE | command: python3 /home/baiyihuan/.codex/skills/.system/skill-creator/scripts/quick_validate.py tron-skills/eliteforge-memory-dreaming | cost: low | when: before commit | rerun: after skill metadata or reference edits | expect: skill validation passes

- CMD-DOCS-SPEC | command: python3 /home/baiyihuan/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/eliteforge-memory-dreaming-prompt-quality-foundation --phase spec --complete-spec | cost: low | when: before implementation and before commit | rerun: after feature spec edits | expect: feature docs pass validator

- CMD-LIVE-SMOKE-DRY | command: python3 tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_live_smoke.py --codex-home ~/.codex --artifact-root /tmp/eliteforge-memory-dreaming-live --sample-size 8 --seed 20260528 --dry-run | cost: manual-private | when: local private live-smoke readiness check | rerun: before manual real-data smoke | expect: sampled plan uses repo-external artifacts and aggregate-only reporting

- CMD-CODEX-CLI-SKILL-EVAL | command: `ARTIFACT_ROOT=/tmp/eliteforge-memory-dreaming-private-eval/20260529-ac11-ac12-v1 CODEX_HISTORY_ROOT="${CODEX_HOME:-$HOME/.codex}" codex exec --cd /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base "<prompt from tron-skills/eliteforge-memory-dreaming/references/private-evaluation.md#proposed-ac11ac12-run-v1>"` | cost: manual-private | when: mandatory final quality gate after implementation and after user approves the exact private prompt, sample scope, artifact root, redaction policy, and review rubric | rerun: after skill prompt changes or failed quality review | expect: real Codex Agent prepares an 8-session sampled Codex home, completes dreaming artifacts from real local history, and returns only sanitized aggregate AC11/AC12 reviewer evidence

# Evidence Rules

- Commit automated test output only as concise command results or reports.
- Do not commit live-smoke artifacts, real session snippets, private absolute paths, or source hashes.
- When live smoke is run, summarize only aggregate stability and failure categories.
- When real Codex CLI evaluation is run, commit only sanitized quality metrics, failure categories, and reviewer conclusions.
- Do not mark the skill complete if AC-MDPQ-11 and AC-MDPQ-12 have only synthetic fixture evidence or helper-script evidence.
- If a prompt/schema change alters expected memory semantics, update semantic manifests before implementation.
