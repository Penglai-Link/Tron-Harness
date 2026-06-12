# RAG Storage Write Planner

## Scope

Upgrade RAG ingest persistence from hard-coded SQLite/Graph branching to a storage write plan compiled from `storageBinding.segmentBindings` and `segmentTargets`.

The ingest pipeline remains responsible for resolving, parsing, splitting, fulltext preparation, embedding, and building `RagPersistIngestCommand`. The persistence layer becomes responsible for compiling target-aware write operations and dispatching them to storage adapters.

## Behavior

The storage planner groups configured segments by target and backend. Shared targets are written once by the owning adapter. Split targets are written as separate operations. Existing SQLite and Graph behavior must remain compatible.

The SQLite adapter supports retrieval-bearing writes through the existing SQL corpus plan. It also supports source-only writes when only the `source` segment is bound to SQLite and `textUnit`, `fulltext`, and `vector` are bound to Graph. Source-only writes must not require a SQLite text-unit, fulltext, or vector table.

The Graph adapter keeps existing single-node and segmented-node behavior. The orchestration layer decides which backend operations exist; the Graph adapter decides whether Graph operations are single-node or segmented.

## Out Of Scope

Search route adapter registry is not included. Retrieval remains route/retriever driven. Cross-backend distributed transactions, rollback compensation, and destructive table migration are also out of scope.
