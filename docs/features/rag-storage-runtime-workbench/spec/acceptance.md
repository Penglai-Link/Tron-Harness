# Acceptance Index

- AC-RSW-01 [must] [backend/database] Creating or updating a Corpus with supported table-backed storage targets creates the required physical storage objects for the configured target tables before the Corpus is reported usable.
- AC-RSW-02 [must] [backend/api] Corpus storage updates reject or surface unsupported storage materialization cases instead of saving configurations that the current runtime cannot write or search.
- AC-RSW-03 [must] [backend/api] Ingest Profile create/update persists runtime-compatible stage, source policy, and corpus binding configuration into profile storage and the saved profile can be loaded by the ingest runtime.
- AC-RSW-04 [must] [backend/api] Retrieval Profile create/update persists runtime-compatible route, filter, policy, and corpus binding configuration into profile storage and the saved profile can be loaded by the search runtime.
- AC-RSW-05 [must] [frontend] The RAG profiles pages save and edit data through backend APIs using the catalog-compatible structure, not browser-only mock state, in API mode.
- AC-RSW-06 [must] [frontend] The RAG workbench page can run document ingest and retrieval search through backend APIs with visible success/error feedback and profile/corpus selectors aligned to saved backend data.
- AC-RSW-07 [must] [end-to-end] A Project Doc corpus/profile configuration saved through the management flow can ingest `docs/features/rag-corpus-storage-profile-foundation/spec/feature-spec.md` and return searchable results through `/api/v1/rag/search`.
- AC-RSW-08 [must] [regression] Existing RAG corpus/profile form tests and backend RAG management tests continue to pass after storage materialization and workbench integration.

## Open Decisions

- Graph storage object creation is validated after code inspection; if current graph repository lacks safe schema/node initialization APIs, AC-RSW-02 will require an explicit unsupported-materialization validation path.
