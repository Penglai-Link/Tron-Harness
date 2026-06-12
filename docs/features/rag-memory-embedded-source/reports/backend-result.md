# Backend Result

## Evidence

- AC-1: `rag_service_contracts::rag_service_resolves_enabled_corpus_storage_binding_from_repository_port` 通过，确认 `memory-card` 使用 `EMBEDDED_SOURCE` 和 `rag_memory_text_units`。
- AC-2: `rag_ingest_contracts::memory_plain_text_ingest_writes_source_text_unit_fulltext_and_vector_fields` 通过，确认 Memory 写入落到合表字段。
- AC-3: `rag_search_contracts`、`rag_diagnostics_contracts`、`rag_diagnostics_api` 通过，确认检索、debug、retry、trace 继续可用。
- AC-4: `rag_foundation_schema` 通过，确认最终 RAG 表清单不包含 `rag_memory_sources`，Memory 治理字段和索引位于 `rag_memory_text_units`。
- AC-5: `rag_ingest_contracts` 和 `rag_search_contracts` 中 Project Doc 用例通过，确认 `project-doc` split 逻辑未回退。

## Final Validation

- `make be-check`: pass
- `make be-test`: pass
