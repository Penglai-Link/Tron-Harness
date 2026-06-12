# Database Design: RAG Skill Graph Policy

## Database Decision

- status: approved
- impact: SQLite RAG profile seed plus graph node/edge property writes
- approved_by: langtaoxin
- approved_at: 2026-06-04

## Existing Schema Impact

- Existing `rag_corpora` and `rag_profiles` remain the configuration source.
- Existing `skill`, memory, and project-doc tables remain unchanged.
- Graph storage continues to store skill graph nodes and edges through the existing graph engine.

## Proposed Schema

No new physical table is added. The existing `skill-graph-ingest-v1` seed keeps `graphWritePolicy.maxRelatedNodes` and `maxRelatedEdges`, and omits `allowedNodeKinds` and `allowedRelationships` by default.

## Relationships And Cardinality

- One skill source/version/scope maps to one main graph node.
- A skill node can have zero or more related nodes and edges.
- Related edges can carry scalar custom properties.

## Migration Plan

- Update the additive `skill-graph` migration seed.
- Existing databases replay the idempotent seed with the updated config.
- No data migration from existing `skill` SQLite corpus is performed.

## Query And Performance Plan

- Relation expansion remains bounded by `graphExpansionPolicy.maxDepth` and `maxRelatedNodes`.
- Write-time related graph size remains bounded by `graphWritePolicy.maxRelatedNodes` and `maxRelatedEdges`.

## Data Governance

- Edge and node custom properties reject objects, arrays, nulls, empty keys, and reserved technical keys.
- RAG technical fields remain fixed and are not overwritten by domain fields.

## Test Data And Fixtures

- Tests use temporary SQLite database files.
- Tests use temporary local Grafeo graph storage files.
- Tests use deterministic `FakeRuntime` embedding and rerank behavior.

## Validation Commands

- `make be-check`
- `make be-test`
