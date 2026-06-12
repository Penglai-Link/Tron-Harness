# Acceptance: RAG Skill Graph Corpus

## Acceptance Index

- AC-RSG-01 [must] [regression] Existing skill, memory-card, and project-doc corpora keep their configured storage and search behavior; proof: regression tests pass.
- AC-RSG-02 [must] [database] Skill-graph seed adds an enabled corpus and profiles using graph single-node storage; proof: migration and config tests pass.
- AC-RSG-03 [must] [database] Skill-graph ingest writes one scoped RagSkill graph node with status fields and reused embedding; proof: graph ingest tests pass.
- AC-RSG-04 [must] [database] Typed relatedGraph payload writes whitelisted related nodes and edges idempotently; proof: related graph tests pass.
- AC-RSG-05 [must] [business] Skill-graph fulltext and vector routes return scoped CandidateHit rows through existing ranking; proof: graph search tests pass.
- AC-RSG-06 [must] [business] Dedup keys isolate corpus, source, source version, scope, and target identities; proof: ranking tests pass.
- AC-RSG-07 [must] [business] Graph relation expansion runs after rerank and final limit only for final graph hits; proof: expansion order tests pass.
- AC-RSG-08 [must] [business] Debug and retry route graph-backed sources without SQLite table-plan errors; proof: diagnostics and retry tests pass.
- AC-RSG-09 [must] [regression] Embedding and rerank verification uses fake runtimes and no external provider network; proof: test evidence shows fake runtime.

## Open Decisions

- None.
