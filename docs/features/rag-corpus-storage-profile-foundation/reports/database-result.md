# Database Report: RAG 后端底座 - Corpus 存储与 Profile 基础

## Summary

数据库子任务已完成并集成到 base 分支。范围限定在 RAG foundation migration、SQLite migration registry、SQLite RAG repository schema/seed 检查和相关测试。

## Evidence Rows

- AC-RAG-01 | red: `make be-test` 在实现前因缺少 `SqliteRagRepository` 失败 | green: `make be-test` 通过，`rag_foundation_migration_creates_approved_tables_and_seeds_corpora_idempotently` 覆盖 7 表和 3 个 Corpus seed | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-02 | red: `make be-test` 在实现前无法检查 RAG foundation schema | green: `make be-test` 通过，`rag_foundation_keeps_source_and_text_unit_storage_split_by_corpus` 覆盖 Source/TextUnit 按 Corpus 分表，全文和向量字段合并在 TextUnit 表 | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.
- AC-RAG-08 | red: `make be-test` 在实现前缺少 scope 字段模型和 schema 证据 | green: `make be-test` 通过，`rag_foundation_preserves_scope_isolation_columns_without_permission_columns` 覆盖 `scope_type` 和 `scope_code`，且未加入权限列 | command_ref: CMD-BE-TEST | browser:not applicable | status: passed.

## Changed Paths

- `tron-backend/migrations/20260530_0022_create_rag_foundation.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/rag_repository.rs`
- `tron-backend/src/domain.rs`
- `tron-backend/src/domain/rag.rs`
- `tron-backend/tests/rag_foundation_schema.rs`
- `docs/features/rag-corpus-storage-profile-foundation/reports/agents/database-schema-seed.md`

## Commands

- `make be-test`: passed after database worker merge and again after backend worker merge.

## Blockers

- None.
