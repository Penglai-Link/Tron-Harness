# Backend Service Contracts Report

## Scope

- Agent: backend
- Subtask: service-contracts
- Branch: `feature/langtaoxin/rag-corpus-storage-profile-foundation/backend/service-contracts`
- Feature: `rag-corpus-storage-profile-foundation`

## AC Evidence

### AC-RAG-03

- Added Corpus service projection that resolves enabled Corpus storage binding, field mapping, and filterSchema through `RagService`.
- Proof: `rag_service_resolves_enabled_corpus_storage_binding_from_repository_port` passes in `tron-backend/tests/rag_service_contracts.rs`.

### AC-RAG-04

- Added `RagConfigRepository` port, `RagService`, read-only Corpus/Profile DTOs, and SQLite adapter projection for `rag_corpora` and `rag_profiles`.
- Seeded first read-only retrieval profiles without changing the approved 7-table schema.
- Proof: `rag_service_lists_and_reads_profile_configuration_without_sqlite_details` passes.

### AC-RAG-05

- Retrieval Profile validation rejects missing, blank, and `latest` `requiredSourceVersion` values.
- Proof: `retrieval_profile_validation_rejects_missing_or_latest_source_version` passes.

### AC-RAG-06

- Added finite strategy registry contract with enabled, reserved, and unsupported statuses for stage, retriever, and policy references.
- Proof: `strategy_registry_validation_reports_enabled_reserved_and_unsupported_references` passes.

### AC-RAG-07

- Added filterSchema validation that accepts declared logical fields and rejects unknown fields, invalid value types, and unsupported operators.
- Proof: `filter_validation_accepts_declared_fields_and_rejects_invalid_filters` passes.

### AC-RAG-09

- Added boundary regression coverage confirming no RAG HTTP route registration and no RAG application dependency on `LlmRuntimeService`, `GraphService`, or `EventBusHandle`.
- Proof: `rag_foundation_does_not_register_http_routes_or_call_runtime_graph_or_event_services` passes and `make be-check` passes.

## Changed Paths

- `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`
- `tron-backend/src/application.rs`
- `tron-backend/src/application/rag.rs`
- `tron-backend/src/application/rag/service.rs`
- `tron-backend/src/domain/rag.rs`
- `tron-backend/src/domain/rag/model.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/tests/rag_service_contracts.rs`
- `docs/features/rag-corpus-storage-profile-foundation/reports/agents/backend-service-contracts.md`

## Commands

### RED

- `make be-test`
- Result: failed as expected before implementation.
- Summary: compile failed because `tron_backend::application::rag` and RAG domain contract types such as `RagCorpusRecord`, `RagProfileRecord`, `RagProfileType`, `RagStatus`, and `RagStorageBinding` did not exist.

### GREEN

- `cargo fmt --manifest-path tron-backend/Cargo.toml`
- `make be-test`
- Result: passed.
- Summary: full backend test suite passed, including 6 tests in `tests/rag_service_contracts.rs` and existing RAG schema tests.

### VERIFY

- `make be-check`
- Result: passed.
- Summary: backend compile check passed, supporting AC-RAG-09 boundary proof.

## Residual Risks

- Strategy reference extraction intentionally supports the first profile JSON shapes used by this foundation (`stages`, `retrievers`, `policies`, and related singular/id keys). Future profile config shapes should add focused tests before relying on validation.
- Filter validation normalizes only configuration-level filter JSON; it does not build SQL predicates or enforce permissions, by design for this foundation feature.
