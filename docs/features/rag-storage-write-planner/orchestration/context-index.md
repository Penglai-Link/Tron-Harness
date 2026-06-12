# Context Index

## Feature Documents

- spec/feature-spec.md: scope and behavior
- spec/acceptance.md: AC rows
- spec/test-plan.md: command index and proof rows
- tasks/backend-task.md: implementation slice
- reports/backend-result.md: evidence ledger

## Code Touchpoints

- tron-backend/src/adapters/outbound/persistence/rag_graph_repository.rs
- tron-backend/src/adapters/outbound/persistence/rag_graph_repository/ingest.rs
- tron-backend/src/adapters/outbound/persistence/rag_graph_repository/write_adapters.rs
- tron-backend/src/adapters/outbound/persistence/rag_graph_repository/write_plan.rs
- tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/ingest.rs
- tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/index_schema.rs
- tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository/sql_plan.rs
- tron-backend/tests/rag_skill_graph_contracts.rs
- tron-backend/tests/rag_search_contracts.rs

## Current Evolution

- AC-6 through AC-9 extend the initial mode-based write planner into a generic target-grouped planner.
- Primary edit scope: `write_plan.rs`, `write_adapters.rs`, and focused planner tests.
