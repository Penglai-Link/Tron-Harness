# Acceptance

- AC-1: The persistence layer compiles a write plan from `storageBinding` so current non-Graph SQLite corpora and mixed Graph corpora no longer rely on hard-coded top-level SQLite/Graph branching.
- AC-2: Existing all-SQLite, SQLite carrier, Graph single-node, and Graph segmented ingest/search behaviors remain compatible.
- AC-3: A corpus configured with `source=SQLITE` and `textUnit/fulltext/vector=GRAPH` writes the source row to the configured SQLite source table and writes Graph nodes for the Graph segments.
- AC-4: The write orchestration exposes backend-specific adapters for SQLite and Graph, with source-only SQLite persistence handled without compiling a full SQLite retrieval plan.
- AC-5: Focused RED/GREEN/VERIFY evidence is recorded for planner compatibility and source-only SQLite behavior.
- AC-6: The write plan emits generic target operations grouped by `segmentBindings -> segmentTargets`, including target id and target metadata, without storing backend-specific mode names in the plan.
- AC-7: Backend-specific write-combination rules are owned by backend adapters: SQLite decides retrieval-bearing versus source-only writes, and Graph decides single-target versus segmented writes.
- AC-8: Existing SQLite carrier, source-only SQLite plus Graph, Graph single-node, and Graph segmented ingest/search behaviors remain compatible after the generic planner refactor.
- AC-9: Focused RED/GREEN/VERIFY evidence is recorded for generic target grouping and adapter-owned write-combination behavior.
