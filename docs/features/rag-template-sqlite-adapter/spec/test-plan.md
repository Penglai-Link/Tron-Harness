# Test Plan: RAG 模板化 SQLite Adapter

## Command Index

- CMD-1: `make be-check`
- CMD-2: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts --test rag_service_contracts -- --nocapture`
- CMD-3: `cargo test --manifest-path tron-backend/Cargo.toml --test rag_search_contracts --test rag_diagnostics_contracts --test rag_service_contracts --test rag_foundation_schema -- --nocapture`
- CMD-4: `make be-test`

## Proof Rows

- AC-1: 使用 `rag_service_contracts` 校验 `storage_binding.layout = SPLIT`，并继续读取当前 Source/TextUnit 表。
- AC-2: 使用 `rag_search_contracts::sqlite_rag_repository_uses_template_plans_instead_of_corpus_sql_branches` 校验 repository 不包含具体 Corpus 表名和 corpus SQL 分支。
- AC-3: 使用 `rag_search_contracts` 校验 FULLTEXT/VECTOR route 命中、融合、去重、rerank 和 context pack。
- AC-4: 使用 `rag_diagnostics_contracts` 校验 debug、retry、trace 和 event 行为。
- AC-5: 使用 `rag_foundation_schema` 校验当前表仍按 Corpus 分开，未合表。
