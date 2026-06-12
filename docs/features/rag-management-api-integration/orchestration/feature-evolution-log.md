# Feature Evolution Log: RAG 管理配置 API 接入

## Summary

- feature_slug: rag-management-api-integration
- mode: zero_one_delivery
- status: complete

## Change History

- 2026-06-10: approved phase 1 scope to wire RAG corpus and profile management pages to backend management APIs while preserving current runtime ingest/search contracts.
- 2026-06-10: chose the target-first corpus storage topology model with runtime compatibility projection persisted alongside management storage.
- 2026-06-10: chose separate ingest and retrieval profile forms, with retrieval profiles supporting multiple corpora and mock strategy catalogs aligned to current backend strategy ids.
- 2026-06-10: approved phase 1.1 corpus topology evolution so `source`, `textUnit`, and `vector` can bind to independent SQLite targets in management APIs.
- 2026-06-10: completed phase 1.2 SQLite runtime evolution so optional legacy `vectorTable` compatibility data is honored by ingest/search/debug/retry execution instead of falling back to `textUnitTable`.

## Downstream Artifact Migration

- No downstream runtime contract migration was required because runtime readers continue consuming `storage_binding_json` and existing profile storage fields.
- Phase 1.1 extended the legacy runtime binding payload with an optional `vectorTable` field.
- Phase 1.2 reused that compatibility field to create a separate SQLite vector business table plus vec sidecar storage on demand, without adding new SQL columns or changing ingest/search/debug/retry request contracts.

## Residual Risks

- `make be-test` may still reproduce existing non-RAG auth/MCP baseline failures, so the repository-wide backend baseline can remain not fully green even when the RAG slice is verified.
- The feature still rejects broader mixed SQLite + Graph execution combinations; only the optional split-SQLite `vectorTable` path is evolved in runtime.
