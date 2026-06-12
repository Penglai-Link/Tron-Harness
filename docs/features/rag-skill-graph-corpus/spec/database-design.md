# Database Design: RAG Skill Graph Corpus

## Database Decision

- status: approved
- impact: graph and SQLite seed migrations
- approved_by: langtaoxin
- approved_at: 2026-06-04

## Existing Schema Impact

- Existing SQLite RAG corpus/profile tables remain the configuration source for corpus and profile metadata.
- Existing `rag_skill_text_units`, memory, and project-doc tables remain untouched for existing corpora.
- Graph storage stores `skill-graph` data in the existing scoped graph engine; no new graph HTTP API is added.
- Diagnostics continue to use existing `rag_diagnostics` for warnings and failure metadata.

## Proposed Schema

### rag_corpora seed row

- change_type: create seed
- fields: `corpus_id=skill-graph`, graph storage binding with `layout=GRAPH_SINGLE_NODE`, namespace, node label, code strategy, diagnostics table
- constraints: corpus id unique
- indexes: existing corpus primary key
- notes: additive seed only; does not update `skill`

### rag_profiles seed rows

- change_type: create seed
- fields: `skill-graph-ingest-v1`, `skill-graph-related-v1`, graph route and policy config
- constraints: profile id unique
- indexes: existing profile primary key
- notes: profile configs hold related graph write policy and relation expansion policy

### Graph RagSkill node

- change_type: create graph node
- fields: source compatibility fields, text compatibility fields, status fields, metadata JSON strings, full text, semantic payload
- constraints: node code stable by corpus, source, version, scope, and code strategy
- indexes: existing graph in-memory/full scan adapter behavior; future native graph indexes out of scope
- notes: avoid reserved graph business property names

### Graph related nodes and edges

- change_type: create graph node and edge
- fields: related kind, code, title, text, relationship, properties
- constraints: kind and relationship must pass profile whitelist
- indexes: existing graph traversal structures
- notes: idempotent upsert within current graph scope

## Relationships And Cardinality

- One `skill-graph` source version maps to one `RagSkill` node per target scope.
- A `RagSkill` node can have zero or more whitelisted outgoing related edges.
- Related nodes can be shared within the same graph scope and are identified by normalized graph code.
- Updates replace or upsert relationships for the current skill source/version/scope without deleting unrelated corpora.

## Migration Plan

- Add a new SQLite migration after current RAG skill corpus seed.
- Extend storage binding parser to accept graph layout fields.
- Seed corpus/profile rows idempotently.
- No backfill from `skill` to `skill-graph`.
- Rollback is to remove the additive seed rows and graph data created by test fixtures or callers.

## Query And Performance Plan

- Graph search uses graph label, property filters, full text match, and semantic projection through existing graph store.
- RAG OR scope semantics are implemented by querying USER, AGENT, and SPACE graph scopes independently and merging rows.
- Relation expansion runs only for final result hits after rerank and final limit to bound traversal cost.
- First version caps related nodes and edges by profile policy.

## Data Governance

- Graph namespace isolates RAG skill graph data from other graph domains.
- Scope fields preserve USER, AGENT, and SPACE ownership.
- No raw graph query, raw traversal text, API keys, provider responses, or embeddings are returned through RAG HTTP responses.
- RelatedGraph metadata returned to callers is capped and policy-controlled.

## Test Data And Fixtures

- Tests create temporary SQLite and Grafeo graph stores.
- Tests seed fake `skill-graph` data through migrations or fixture helpers.
- Embedding and rerank tests use deterministic fake vectors and fake rerank ordering.
- Fixtures clean temporary graph storage paths after test process exit.

## Validation Commands

- `make be-test`: run backend tests; expect graph corpus tests and regression tests pass or unrelated existing failure is recorded.
- `make be-check`: compile backend after trait and wiring changes; expect pass or known unrelated architecture guard is recorded.
