# Feature Spec: RAG Multi-Corpus RRF And Topic Strategy

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- user_confirmation_status: approved_by_user

The feature extends the existing RAG search pipeline so one retrieval profile can search multiple corpus tables, fuse route-local candidates with Reciprocal Rank Fusion, deduplicate with a corpus-safe key, and optionally apply a first version topic diversity strategy based on already stored candidate vectors.

## Feature Identity And Context

- feature_slug: rag-rrf-topic-strategy
- feature_name: RAG Multi-Corpus RRF And Topic Strategy
- owner: langtaoxin
- status: approved_with_defaults
- dependency_features: rag-search-context-pack, rag-domain-fields, rag-ingest-dag
- dependency_contracts: tron-backend/src/application/rag/search.rs, tron-backend/src/application/rag/search_profile.rs, tron-backend/src/application/rag/search_ranking.rs, tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs

## Requirement

- user_story: As a RAG caller, I need one retrieval request to combine results from several corpus tables without over-promoting duplicate or near-duplicate topics.
- intent: Add configurable fusion, dedup, and first-version topic diversity policies without changing existing search API behavior for profiles that do not opt in.
- outcome: Multi-corpus retrieval can use rank-based fusion and embedding MMR before rerank, while legacy profiles keep current ordering semantics.

## Product Source Slices

- source_prd: docs/brainstorm/rag-backend-foundation/prd.typ
- source_feature: existing RAG search and context-pack backend foundation
- source_discussion: multi-corpus retrieval plan approved in this thread on 2026-06-03

## Scope Boundary

The slice is backend-only. It changes retrieval profile compilation, application-level candidate ranking, SQLite candidate projection for stored vectors, and RAG contract tests.

## Assumptions

- Candidate text units already persist embeddings when vector indexing succeeds.
- Similar topic grouping first uses existing embeddings and does not call an LLM.
- Topic diversity is applied after fusion and before rerank.
- Fulltext-only candidates may not have embeddings and must keep the fused order.

## In Scope

- Parse optional `fusionPolicy`, `dedupPolicy`, and `topicPolicy` profile config.
- Implement RRF scoring using route-local ranks and route weights.
- Add corpus-safe identity dedup and canonical exact-text dedup.
- Implement `EMBEDDING_MMR_V1` over the configured candidate window.
- Return persisted text-unit embeddings to the ranking layer when available.
- Preserve existing search, filter, trace, context, and HTTP contracts.

## Out Of Scope

- No LLM topic clustering.
- No new database tables or migrations.
- No frontend UI.
- No new public API endpoint.
- No fuzzy semantic dedup beyond embedding MMR ordering.

## Constraints

- Use Makefile commands for backend verification.
- Keep existing RAG response envelope unchanged.
- Do not persist raw embeddings or provider payloads in search traces.
- Do not collapse unrelated cross-corpus rows that only share a local target code.

## Feature Model

- `FusionPolicy`: defaults to `SUM_SCORE`; opt-in `RRF` computes score from route rank and route weight.
- `DedupPolicy`: defaults to text-unit identity; opt-in canonical exact-text grouping merges identical normalized text.
- `TopicPolicy`: defaults to disabled; opt-in `EMBEDDING_MMR_V1` reorders the candidate window by relevance and vector diversity.
- `RankedCandidate`: carries the public hit plus route ranks, optional embedding, identity key, and canonical dedup key.
- `Representative`: the candidate kept for a dedup group while route rank metadata and scores are merged.

## Page And Module Surface

- Backend application: tron-backend/src/application/rag/search.rs
- Profile compiler: tron-backend/src/application/rag/search_profile.rs
- Ranking pipeline: tron-backend/src/application/rag/search_ranking.rs
- SQLite search adapter: tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/search.rs
- Backend contracts: tron-backend/tests/rag_search_contracts.rs

## Repository Findings

- Retrieval profiles already support route arrays and corpus IDs, so the change can extend profile config instead of adding a new endpoint.
- RAG search already executes route fan-out and rerank, making the safest insertion point after route fusion and before rerank.
- SQLite vec0 tables are good for retrieval, but stored text-unit embeddings are the stable source for application-level MMR.
- Existing diagnostics reject unsafe trace payload keys, so ranking trace metadata must stay lightweight and avoid raw vectors.

## Open Decisions

- None.
