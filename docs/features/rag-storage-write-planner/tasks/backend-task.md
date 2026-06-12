# Backend Task

## AC Coverage

- AC-1: Introduce write plan model and planner.
- AC-2: Preserve current SQLite/Graph behavior.
- AC-3: Add source-only SQLite persistence for Graph-backed corpora.
- AC-4: Introduce SQLite and Graph write adapter dispatch.
- AC-5: Record evidence.
- AC-6: Refactor write plan operations to generic target-grouped operations.
- AC-7: Move backend write-combination branching into backend adapters.
- AC-8: Preserve existing focused ingest/search behavior.
- AC-9: Record evidence for the generic planner refactor.

## Execution

phase: tdd

The implementation should keep ingest pipeline artifacts unchanged and limit behavior changes to persistence planning and adapter dispatch. The generic planner refactor must remove backend-specific mode names from the plan shape and keep SQLite/Graph combination rules inside backend adapters.
