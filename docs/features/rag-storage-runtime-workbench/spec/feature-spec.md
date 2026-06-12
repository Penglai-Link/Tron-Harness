# RAG Storage Runtime Workbench

## Feature Boundary

This feature evolves the RAG management module from configuration-only forms into a runnable management flow. Corpus storage edits must materialize supported physical storage targets, profile edits must persist runtime-compatible ingest and retrieval profiles, and the RAG workbench must execute ingest and search through backend APIs.

## Product Source

The user-approved scope is the current RAG release branch plus the explicit request to:

- create storage objects when Corpus table names or table-related structure fields change,
- persist profile forms into backend profile tables with runtime-compatible configuration,
- verify saved profile data through RAG ingest and search APIs,
- complete the RAG workbench write/search flow and adjust the page when needed.

## Implementation Boundary

- Supported storage materialization starts with SQLite table targets that the current backend runtime can read and write.
- Graph storage targets must not be silently accepted if the current runtime cannot create the required namespace/node shape.
- Existing unified response envelopes remain `code`, `message`, and `data`.
- Existing RAG ingest and search runtime APIs are reused; this feature should not invent a parallel runtime path.

## Out Of Scope

- Rebuilding the entire graph storage runtime.
- Adding destructive table rename or data migration behavior unless required for safe create/update.
- Replacing the existing RAG profile strategy registry.
- Changing unrelated RAG corpus form layout decisions already accepted in the release branch.

## Dependencies

- RAG corpus/profile management APIs.
- SQLite RAG repository migrations and runtime repositories.
- RAG ingest API and RAG search API.
- Frontend RAG management and workbench pages.
