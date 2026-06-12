# Feature Evolution Log: RAG Skill Graph Corpus

## Summary

- feature_slug: rag-skill-graph-corpus

This feature evolves the approved RAG skill corpus plan from SQLite-only skill storage to an additive graph-backed `skill-graph` corpus while preserving all existing corpora.

## Change History

| date | change | reason | impact |
| --- | --- | --- | --- |
| 2026-06-04 | Created feature workspace from current approved conversation | User requested codex-superpower implementation | Establishes AC, TDD, worktree, and evidence plan |

## Downstream Artifact Migration

- No downstream API or frontend artifacts require migration.
- Existing `skill` corpus stays active and unchanged.
- Callers opt into graph behavior by selecting `skill-graph` corpus/profile ids.

## Residual Risks

- Graph adapter currently loads visible graph records in memory, so large-scale native graph indexes remain future work.
- Cross-store atomicity between SQLite config/diagnostics and graph data remains best-effort for this first version.
