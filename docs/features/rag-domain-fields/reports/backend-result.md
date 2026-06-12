# Backend Result: RAG 后端底座 - Memory 与 Project Doc 领域字段

## Summary

- status: completed
- worker_branch: `feature/langtaoxin/rag-domain-fields/backend/domain-fields`
- scope: 仅实现 RAG domain fields schema evolution、Corpus seed 映射和 filter validation 测试；未新增 ingest/search/HTTP/model/graph/event/Memory flow 行为。

## Commands

- RED: `make be-test` -> failed，新增 schema 测试先失败：`rag_memory_sources should expose memory_type`，`rag_project_doc_sources should expose repo_code`；Cargo 在 `rag_foundation_schema` 失败后停止，未继续执行后续 service contract suite。
- format: `cargo fmt --manifest-path tron-backend/Cargo.toml` -> passed。
- GREEN: `make be-test` -> passed；`rag_foundation_schema` 5 tests passed，`rag_service_contracts` 11 tests passed，backend 全量测试 passed。
- VERIFY: `make be-check` -> passed。
- VERIFY: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-domain-fields --phase spec --spec-stage test-plan --complete-spec --no-downstream-artifacts` -> failed；原因是当前 workspace 已存在 `tasks/` 和 `orchestration/` artifacts，与该 flag 的 0-1 spec gate 前置假设冲突。
- VERIFY: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-domain-fields --phase spec --spec-stage test-plan --complete-spec` -> passed。

## Evidence Rows

- AC-RDF-01 | red: `make be-test` failed because `memory_type` was missing | green: `make be-test` passed with Memory columns and indexes present | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RDF-02 | red: `make be-test` failed because `repo_code` was missing | green: `make be-test` passed with Project Doc columns and indexes present | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RDF-03 | red: service test was added before seed update and old seed lacked Memory governance mappings | green: `memory_corpus_seed_maps_governance_logical_fields_to_physical_columns` passed | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RDF-04 | red: service test was added before seed update and old seed lacked Project Doc repo/module mappings | green: `project_doc_corpus_seed_maps_repo_module_logical_fields_to_physical_columns` passed | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RDF-05 | red: filter validation test was added before seed update and old filterSchema lacked new domain fields | green: `filter_validation_accepts_domain_fields_and_rejects_unknown_or_mistyped_values` passed | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RDF-06 | red: existing boundary test retained the no HTTP/model/graph/event guard | green: boundary test passed and `make be-check` passed | command_ref: CMD-BE-CHECK | browser:not applicable | status: passed.

## Changed Paths

| path | ac_refs | result |
| --- | --- | --- |
| `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql` | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05 | adds RAG domain indexes and updates `memory-card` / `project-doc` seed mappings |
| `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs` | AC-RDF-01, AC-RDF-02 | registers the domain fields migration asset |
| `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs` | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04 | runs foundation plus idempotent domain field schema evolution and seed update on connect |
| `tron-backend/tests/rag_foundation_schema.rs` | AC-RDF-01, AC-RDF-02 | covers new columns, indexes, and repeated SQLite initialization |
| `tron-backend/tests/rag_service_contracts.rs` | AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 | covers logical field mappings, filter validation, and behavior boundary |
| `AGENTS.md` | AC-RDF-06 | records the docs validation flag mismatch encountered during VERIFY |
| `docs/features/rag-domain-fields/reports/backend-result.md` | AC-RDF-01, AC-RDF-02, AC-RDF-03, AC-RDF-04, AC-RDF-05, AC-RDF-06 | records worker evidence |

## AC-RDF-01

- changed_paths: `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/tests/rag_foundation_schema.rs`
- RED: `make be-test` failed on `rag_domain_fields_migration_adds_memory_governance_columns_and_indexes_idempotently` because `memory_type` was absent before the domain fields migration.
- GREEN: `make be-test` passed after `SqliteRagRepository::connect` ran foundation migration, column existence checks, domain indexes, and seed update.
- VERIFY: Memory Source exposes `memory_type`, `memory_status`, `review_status`, `version`, `superseded_by`, `conflict_group`, `duplicate_group`, `injectable`, `importance_score`, `confidence_score`, `quality_score`, `pin_level`, `expires_at`, `derived_source_type`, `derived_source_code` plus indexes `idx_rag_memory_sources_domain_status`, `idx_rag_memory_sources_domain_rank`, `idx_rag_memory_sources_expires`.

## AC-RDF-02

- changed_paths: `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/tests/rag_foundation_schema.rs`
- RED: `make be-test` failed on `rag_domain_fields_migration_adds_project_doc_repo_module_columns_and_indexes_idempotently` because `repo_code` was absent before the domain fields migration.
- GREEN: `make be-test` passed after Project Doc columns and indexes were added idempotently.
- VERIFY: Project Doc Source exposes `repo_code`, `module_code`, `module_name`; keeps `document_code`, `document_type`, `path`, `title`, `body_hash`; exposes indexes `idx_rag_project_doc_sources_repo_module` and `idx_rag_project_doc_sources_path`.

## AC-RDF-03

- changed_paths: `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/tests/rag_service_contracts.rs`
- RED: 新增 service contract 测试在 implementation 前写入；第一次 `make be-test` 因 AC-RDF-01/02 schema failures 提前停止，未执行到该 suite。旧 seed 中 Memory filterSchema/fieldMapping 仅声明 `scopeType`、`scopeCode`、`sourceCode`、`status` 等 foundation 字段。
- GREEN: `memory_corpus_seed_maps_governance_logical_fields_to_physical_columns` passed。
- VERIFY: `memory-card` fieldMapping/filterSchema 声明 Memory logical fields 到 physical columns；integer score/rank/time fields 使用 `RagFilterValueType::Integer`，`injectable` 使用 `RagFilterValueType::Boolean`。

## AC-RDF-04

- changed_paths: `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`, `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`, `tron-backend/tests/rag_service_contracts.rs`
- RED: 新增 service contract 测试在 implementation 前写入；第一次 `make be-test` 因 AC-RDF-01/02 schema failures 提前停止，未执行到该 suite。旧 seed 中 Project Doc filterSchema/fieldMapping 未声明 `repoCode`、`moduleCode`、`moduleName` 和 `documentType`。
- GREEN: `project_doc_corpus_seed_maps_repo_module_logical_fields_to_physical_columns` passed。
- VERIFY: `project-doc` fieldMapping/filterSchema 声明 `documentCode`、`documentType`、`path`、`repoCode`、`moduleCode`、`moduleName` 到对应 physical columns。

## AC-RDF-05

- changed_paths: `tron-backend/migrations/20260530_0023_add_rag_domain_fields.sql`, `tron-backend/tests/rag_service_contracts.rs`
- RED: 新增 filter validation 测试在 implementation 前写入；第一次 `make be-test` 因 AC-RDF-01/02 schema failures 提前停止，未执行到该 suite。旧 filterSchema 未声明新 domain fields。
- GREEN: `filter_validation_accepts_domain_fields_and_rejects_unknown_or_mistyped_values` passed。
- VERIFY: `validate_filters` accepts Memory fields such as `memoryType`, `reviewStatus`, `injectable`, `pinLevel`, `importanceScore`, `derivedSourceCode`; accepts Project Doc fields such as `repoCode`, `moduleCode`, `moduleName`, `documentType`, `path`; rejects `metadata.reviewReason`, wrong integer/boolean values, and unsupported operators.

## AC-RDF-06

- changed_paths: `tron-backend/tests/rag_service_contracts.rs`, `AGENTS.md`
- RED: 既有 boundary test 保持覆盖；本次没有先制造 forbidden route/dependency failure。
- GREEN: `rag_foundation_does_not_register_http_routes_or_call_runtime_graph_or_event_services` passed in `make be-test`。
- VERIFY: `make be-check` passed；source changes stayed in SQLite migration/repository and RAG tests. No HTTP route, ingest/search implementation, model call, graph call, event bus call, or Memory flow logic was added.

## Notes

- 反直觉点已追加到 `AGENTS.md`：test-plan 中带 `--no-downstream-artifacts` 的 docs validation command 与当前已有 `tasks/`、`orchestration/` 的 worker workspace 不兼容。

## Blockers

- None.
