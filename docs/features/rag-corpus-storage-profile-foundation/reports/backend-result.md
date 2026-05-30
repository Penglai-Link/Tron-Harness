# Backend Report: RAG 后端底座 - Corpus 存储与 Profile 基础

## Summary

后端子任务已完成并集成到 base 分支。范围限定在 RAG domain/application/repository port、只读配置服务、Profile 校验、filterSchema 校验和边界回归。

## Evidence Rows

- AC-RAG-03 | red: `make be-test` 在实现前缺少 Corpus binding projection | green: `make be-test` 通过，`rag_service_resolves_enabled_corpus_storage_binding_from_repository_port` 覆盖 enabled Corpus 的 storage binding、fieldMapping 和 filterSchema | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-04 | red: `make be-test` 在实现前缺少 `RagService`、DTO 和 repository port | green: `make be-test` 通过，`rag_service_lists_and_reads_profile_configuration_without_sqlite_details` 覆盖 Corpus/Profile 只读投影且不暴露 SQLite 细节 | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-05 | red: `make be-test` 在实现前未校验 `requiredSourceVersion` | green: `make be-test` 通过，`retrieval_profile_validation_rejects_missing_or_latest_source_version` 覆盖缺失、空值和 `latest` 被拒绝 | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-06 | red: `make be-test` 在实现前缺少 strategy registry 校验 | green: `make be-test` 通过，`strategy_registry_validation_reports_enabled_reserved_and_unsupported_references`、`reserved_profile_allows_reserved_strategy_references`、`enabled_profile_still_rejects_reserved_strategy_references` 覆盖 enabled、reserved、unsupported 策略引用 | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-07 | red: `make be-test` 在实现前未校验 filterSchema | green: `make be-test` 通过，`filter_validation_accepts_declared_fields_and_rejects_invalid_filters` 覆盖未知字段、类型错误和不支持操作符被拒绝 | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-09 | red: `make be-test` 在实现前缺少边界回归测试 | green: `make be-check` 通过，且 `rag_foundation_does_not_register_http_routes_or_call_runtime_graph_or_event_services` 确认未新增 RAG HTTP route，未调用 `LlmRuntimeService`、`GraphService`、`EventBusHandle` | command_ref: CMD-BE-CHECK | browser:not applicable | status: passed.

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

- `make be-test`: passed after backend worker merge.
- `make be-check`: passed after backend worker merge.

## Blockers

- None.
