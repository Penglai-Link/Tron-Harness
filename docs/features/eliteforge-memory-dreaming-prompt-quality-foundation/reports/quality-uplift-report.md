# Summary

Quality uplift implemented after the first private real-history evaluation showed that the flow completed but extracted memories were too thin and hard to review.

# Evidence Rows

- AC-MDPQ-13 | red: prior importer used `single-session-material`, so a long Codex session could become one huge Segment task and lose recall during compression | green: importer now uses bounded message-window segmentation, records segment event ids/ranges, and creates one Segment task per segment; Segment and Session outputs require `candidateSignals`, discarded findings, correction timeline, and `coverageStats` | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: pass.

- AC-MDPQ-14 | red: prior EvidenceRef accepted only session/segment/source file/hash fields, which forced reviewers to search large materials manually | green: task validation and evaluator now require event ids, event ordinal range, source line range, evidence summary, support type, claim, and content hash, and reject missing locality | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | status: pass.

- AC-MDPQ-15 | red: prior Light points could be short one-line memories with generic governance hints and no usage boundaries | green: Light points now require `applyWhen`, `doNotApplyWhen`, `usageGuidance`, `counterExamples`, `qualityWarnings`, target scope, subject, purpose, and MemoryValueModel scoring components | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: pass.

- AC-MDPQ-16 | red: prior quality evaluator did not reject missing usage context or whole-source-only traceability | green: quality evaluator passes three golden scenarios and rejects ten semantic mutants, including missing `applyWhen` and missing `sourceLineRange` | command_ref: CMD-SKILL-MEMORY-DREAMING-QUALITY | status: pass.

- AC-MDPQ-17 | red: prior private evaluation rubric could pass on task counts and redaction even when memory content was thin | green: private evaluation reference now requires content richness, traceability coverage, recall/compression pressure, no-op/no-memory reasoning, privacy redaction, and sanitized aggregate reporting for the next real-history evaluation | command_ref: CMD-CODEX-CLI-SKILL-EVAL | status: prepared.

# Commands

- `make skill-memory-dreaming-test` - pass, 37 tests ran OK after the quality-uplift contract.
- `make skill-memory-dreaming-quality-test` - pass, 10 tests ran OK after richer fixtures and evaluator checks.

# Blockers

- A new private real Codex CLI evaluation should be rerun before claiming final quality after these prompt/schema changes, because previous AC-MDPQ-11/12 evidence predates the quality-uplift contract.
