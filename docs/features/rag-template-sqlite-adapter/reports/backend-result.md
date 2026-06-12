# Backend Result

## Evidence

- AC-1: `make be-check` 通过；`rag_service_contracts` 校验 `storage_binding.layout = SPLIT`。
- AC-2: `rag_search_contracts::sqlite_rag_repository_uses_template_plans_instead_of_corpus_sql_branches` 通过。
- AC-3: `rag_search_contracts` 全部通过，覆盖 FULLTEXT/VECTOR route、fusion、dedup、rerank、context pack。
- AC-4: `rag_diagnostics_contracts` 与 `rag_diagnostics_api` 通过，覆盖 debug、retry、trace、event。
- AC-5: `rag_foundation_schema` 通过，确认当前仍保持 Source/TextUnit 按 Corpus 分表。

## Final Validation

- `make be-check`: pass
- `make be-test`: pass
