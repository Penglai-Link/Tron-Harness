# Acceptance: RAG 管理配置 API 接入

## Acceptance Index

- AC-RAGCFG-01 [must] [api] Backend corpora list/detail endpoints expose management DTOs with grouped field mappings, filter schema, target-first storage topology, and preserved metadata groups; proof: focused RAG backend tests pass.
- AC-RAGCFG-02 [must] [business] Backend corpus catalog and verify endpoints expose the storage, field, capability, and compatibility rules required by the current corpus form; proof: focused RAG backend tests pass.
- AC-RAGCFG-03 [must] [business] Backend corpus create/update preserves management topology and metadata groups, keeps optional `vectorTable` compatibility data for split SQLite vector targets, and rejects unsupported topology combinations; proof: focused RAG backend tests pass.
- AC-RAGCFG-04 [must] [api] Backend profile list/detail endpoints expose ingest and retrieval management DTOs, including retrieval multi-corpus configuration; proof: focused RAG backend tests pass.
- AC-RAGCFG-05 [must] [business] Backend profile catalog, filter validation, verify, create, and update flows validate corpus, strategy, filter, and policy references before save and keep runtime-readable profile storage intact; proof: focused RAG backend tests pass.
- AC-RAGCFG-06 [must] [interaction] Frontend corpora pages load, validate, create, and edit through RAG management APIs instead of local mock persistence; proof: frontend checks and RAG corpus browser flow pass.
- AC-RAGCFG-07 [must] [interaction] Frontend ingest and retrieval profile pages load, validate, create, and edit through RAG management APIs, including retrieval multi-corpus configuration; proof: frontend checks and RAG profile browser flow pass.
- AC-RAGCFG-08 [must] [interaction] Unsupported graph/storage combinations surface through the shared corpus form validation flow with backend-backed feedback rather than silent local acceptance; proof: RAG corpus browser flow and focused backend tests pass.
- AC-RAGCFG-09 [must] [regression] Runtime RAG routes still compile and split-SQLite corpora honor optional `vectorTable` across ingest/search/debug/retry without changing current request/response contracts; proof: backend compile and focused RAG runtime regressions pass.
- AC-RAGCFG-10 [must] [interaction] Corpus create/edit storage binding treats `fulltext` as an independent configurable segment beside `source`, `textUnit`, and `vector`, with legacy corpora defaulting `fulltext` to the current text-unit target; proof: frontend checks and focused RAG corpus browser flow pass.
- AC-RAGCFG-11 [must] [api] Corpus management APIs accept, preserve, validate, and return target-first storage topology with `fulltext` bindings, while preserving backward compatibility for payloads that omit `fulltext`; proof: focused RAG management backend tests pass.
- AC-RAGCFG-12 [must] [business] SQLite runtime supports an optional independent `fulltextTable` carrier, writes carrier rows during ingest, builds/rebuilds FTS indexes on that carrier, searches through the carrier, and hydrates results from `textUnitTable`; proof: focused RAG ingest/search/diagnostics/retry tests pass.
- AC-RAGCFG-13 [must] [business] Runtime repository dispatch is role-aware instead of whole-corpus backend-aware, so SQLite source/textUnit storage can be combined with Graph fulltext and/or vector retrieval carriers without breaking canonical text-unit hydration; proof: mixed-backend runtime tests pass.
- AC-RAGCFG-14 [must] [business] Graph carrier targets and graph capability validation support role-aware storage without forcing all master data into Graph; proof: focused Graph-backed RAG tests pass.
- AC-RAGCFG-15 [must] [regression] Role-aware storage keeps existing non-split corpora compatible: embedded SQLite, split SQLite without independent carriers, split SQLite with independent vector only, and Graph single-node legacy corpora continue to ingest/search/debug/retry; proof: focused regression matrix passes.

## Open Decisions

- None.
