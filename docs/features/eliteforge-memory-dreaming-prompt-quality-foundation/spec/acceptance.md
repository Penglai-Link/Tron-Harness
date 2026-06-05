# Acceptance Index

- AC-MDPQ-01 [must] [business] Task details expose versioned prompts and hashes for every task type; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-02 [must] [business] V2 artifacts require provenance, EvidenceRef, empty Light handling, and point-level REM; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-03 [must] [security] Runtime artifacts, previews, temp files, and live smoke outputs are protected from leaks; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-04 [must] [regression] Manifest evaluator passes golden fixtures and fails semantic mutants; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-05 [must] [business] Real Codex live smoke samples private data repeatably and reports aggregates only; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-06 [must] [regression] Skill metadata and prompt references validate through Makefile-first project checks; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-07 [must] [security] REM and Deep remain governance-only in V1 and reject Memory MCP calls; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-08 [must] [business] Skill instructions guide a real Codex Agent through memory dreaming instead of only documenting helper scripts; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-09 [must] [business] Task prompts constrain high-quality extraction, evidence, correction handling, scope boundaries, and memory pollution prevention; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-10 [must] [business] REM and Deep produce explicit no-MCP dry-run governance decisions and escalation reasons when memory work exists; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-11 [must] [business] Final quality acceptance includes a private real Codex CLI session using the skill against sampled local historical Codex data and completing the dreaming flow with reviewable outputs; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-12 [must] [security] Mandatory real Codex CLI evaluation evidence is sanitized to quality metrics and failure categories only, with no raw session text, private paths, session ids, or source hashes committed; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-13 [must] [business] Long Codex sessions are split into bounded segment tasks and Segment/Session outputs preserve candidateSignals and coverageStats before compression; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-14 [must] [business] EvidenceRef is claim-level and reviewable through event ids, event ordinal range, source line range, evidence summary, support type, claim, and content hash instead of whole-segment-only citation; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-15 [must] [business] Light memory points are operationally usable with memoryType, typeSpecific, applyWhen, doNotApplyWhen, usageGuidance, counterExamples, qualityWarnings, and scored governanceHints; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-16 [must] [regression] Quality fixtures and evaluator reject thin memories, missing usage context, missing memoryType/typeSpecific, mismatched type/category, missing claim-level evidence, source pollution, stale corrections, wrong scope, and no-memory false positives; proof: @ref:spec/test-plan.md#proof-rows.

- AC-MDPQ-17 [must] [business] Real Codex CLI quality review must judge content richness, traceability coverage, recall/compression pressure, and no-op/no-memory reasoning in addition to task completion and privacy redaction; proof: @ref:spec/test-plan.md#proof-rows.

# Open Decisions

- none
