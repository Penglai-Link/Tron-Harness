# Acceptance: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Acceptance Index

- AC-RDF-01 [must] [database] Memory Source schema exposes approved governance columns and indexes; proof: schema migration tests pass.
- AC-RDF-02 [must] [database] Project Doc Source schema exposes repo and module columns and indexes; proof: schema migration tests pass.
- AC-RDF-03 [must] [business] Corpus seed maps Memory logical fields to physical columns in filterSchema and fieldMapping; proof: service tests pass.
- AC-RDF-04 [must] [business] Corpus seed maps Project Doc logical fields to physical columns in filterSchema and fieldMapping; proof: service tests pass.
- AC-RDF-05 [must] [api] Filter validation accepts new declared fields and rejects invalid values; proof: filter validation tests pass.
- AC-RDF-06 [must] [regression] Domain fields add no ingest, search, HTTP, model, graph, event, or Memory flow behavior; proof: boundary tests pass.

## Open Decisions

- None.
