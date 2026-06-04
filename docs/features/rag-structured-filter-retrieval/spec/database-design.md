# Database Design: RAG Structured Filter Retrieval

## Database Decision

- status: approved
- impact: repository_read_path_only
- approved_by: langtaoxin
- approved_at: 2026-06-04

No schema migration is required. The feature reads existing SQLite RAG corpus tables through the current `SqliteRagCorpusPlan` and does not create new indexes, tables, registry rows, or materialized views.

## Existing Schema Impact

- Reuses existing `rag_corpora` and `rag_profiles`.
- Reuses existing corpus-specific Source and TextUnit tables such as `rag_memory_sources`, `rag_memory_text_units`, `rag_project_doc_sources`, and `rag_project_doc_text_units`.
- Reuses existing filter-field mappings and return-field mappings.
- Does not modify graph, model, workspace, user, task, chat, or event tables.

## Proposed Schema

No proposed schema changes.

The new retriever is a read-path strategy. It queries canonical TextUnit rows and optional Source rows already described by corpus storage bindings.

## Relationships And Cardinality

- One Retrieval Profile can contain one or more structured-filter routes.
- One structured-filter route targets one configured Corpus.
- One Source version can own many TextUnits.
- One search request can return many TextUnit hits, then existing dedup and fusion reduce duplicates.
- Context Pack remains a response-only structure and is not persisted.

## Migration Plan

- No migration file is added.
- Strategy registration happens in Rust code.
- Existing seed data can add structured-filter routes in future migrations, but this feature does not require default profile seed changes.

## Query And Performance Plan

`STRUCTURED_FILTER_SQLITE_V1` applies:

- source version predicate
- active source and text-unit status predicates
- direct USER, AGENT, and SPACE scopes
- merged profile, route, and request filters
- stable `source_code ASC, ordinal ASC` ordering
- route limit

The first version intentionally avoids total counts and configurable ordering.

## Data Governance

- No query embedding is generated or persisted.
- No provider call is made for blank-query structured-only retrieval.
- Existing filter schema validation remains authoritative.
- Existing source-version resolution remains authoritative.
- Search remains read-only for RAG storage.

## Test Data And Fixtures

- Memory fixtures from `rag_search_contracts.rs` are reused for active, inactive, scoped, and versioned rows.
- Focused tests add a structured-filter Retrieval Profile fixture.
- HTTP tests add a missing-query request fixture for a structured-filter profile.
- Fake runtime counters or existing fake runtime queues prove embedding/rerank are not called in blank-query mode.

## Validation Commands

- `make be-test TEST=rag_search_contracts`
- `make be-test TEST=rag_search_api`
- `make be-check`
- `make be-test`
- `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-structured-filter-retrieval --phase final`
