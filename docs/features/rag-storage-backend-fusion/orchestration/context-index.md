# Context Index: RAG Storage Backend Fusion

## Key Files

- `spec/feature-spec.md` defines scope and boundaries.
- `spec/acceptance.md` owns AC rows.
- `spec/test-plan.md` owns proof rows and command aliases.
- `tasks/backend-task.md` owns execution rows.
- `orchestration/worktree-plan.md` records git/worktree dispatch.
- `reports/backend-result.md` records final evidence.

## Code Surfaces

- Storage model and parsing: `tron-backend/src/domain/rag/model.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- Search orchestration: `tron-backend/src/application/rag/search.rs`
- Request filters: `tron-backend/src/application/rag/search_filter.rs`, `tron-backend/src/application/rag/search_validation.rs`
- Graph RAG adapter: `tron-backend/src/adapters/outbound/persistence/rag_graph_repository.rs`, `tron-backend/src/adapters/outbound/persistence/rag_graph_repository/search.rs`
- Seeds: `tron-backend/migrations/*rag*.sql`
- Tests: `tron-backend/tests/rag_foundation_schema.rs`, `tron-backend/tests/rag_service_contracts.rs`, `tron-backend/tests/rag_search_contracts.rs`, `tron-backend/tests/rag_skill_graph_contracts.rs`
