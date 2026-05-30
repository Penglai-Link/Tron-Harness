# Acceptance: RAG 后端底座 - Corpus 存储与 Profile 基础

## Acceptance Index

- AC-RAG-01 [must] [database] RAG foundation migration creates the 7 approved tables and idempotently seeds `memory-card`, `project-doc`, and reserved `code-graph`; proof: migration and seed tests pass.
- AC-RAG-02 [must] [database] Source and TextUnit storage stays split by Corpus while fulltext and vector fields live inside each Corpus TextUnit table; proof: schema inspection and repository tests pass.
- AC-RAG-03 [must] [business] Corpus storage binding maps each enabled Corpus to its Source table, TextUnit table, diagnostics table, field mapping, and filterSchema; proof: service tests pass.
- AC-RAG-04 [must] [api] `RagService` lists and reads Corpus/Profile configuration without exposing SQLite details to application callers; proof: service and repository tests pass.
- AC-RAG-05 [must] [business] Retrieval Profiles require a concrete `requiredSourceVersion` and never accept `latest` as runtime storage semantics; proof: profile validation tests pass.
- AC-RAG-06 [must] [business] Strategy registry validation detects enabled, reserved, and unsupported stage, retriever, and policy references in Profiles; proof: registry validation tests pass.
- AC-RAG-07 [must] [api] Filter validation accepts only declared logical fields and rejects unknown fields, invalid value types, and unsupported operators; proof: filterSchema validation tests pass.
- AC-RAG-08 [must] [security] RAG foundation preserves `scope_type` and `scope_code` isolation fields without implementing corpus, source, repo, role, or field permissions; proof: model and schema tests pass.
- AC-RAG-09 [must] [regression] This feature adds no RAG HTTP routes and does not call `LlmRuntimeService`, `GraphService`, or `EventBusHandle`; proof: compile and boundary tests pass.

## Open Decisions

- None.
