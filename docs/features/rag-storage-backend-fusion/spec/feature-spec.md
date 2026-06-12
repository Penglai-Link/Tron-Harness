# Feature Spec: RAG Storage Backend Fusion

## Scope

Implement a RAG search evolution that makes corpus storage backends explicit, supports one retrieval profile spanning graph-backed skills plus SQLite-backed memories and project documents, lets request filters apply only to corpora that declare the matching fields, and wires graph vector routes to graph semantic projection without changing the underlying graph store.

## Architecture

RAG corpus storage binding gains an explicit `backend` field. The only supported values for this feature are `SQLITE` and `GRAPH`. Existing SQLite corpora keep their current layouts under `SQLITE`; the graph skill corpus keeps `GRAPH_SINGLE_NODE` under `GRAPH`.

`GraphBackedRagRepository` dispatches by backend first. SQLite-backed corpora continue through the SQLite RAG repository. Graph-backed corpora enter the graph RAG adapter, which then dispatches by graph layout. This keeps future graph layouts local to the graph RAG adapter.

RAG search keeps one query embedding result per request when any route needs vector search. The resolved embedding model code travels with the embedding to route repositories. SQLite routes continue to use the embedding vector as before. Graph routes convert the same embedding into a `PreparedGraphSearchQuery` semantic projection whenever the route query carries an embedding.

Request filters are merged per route. Profile and route filters remain strict configuration contracts. Request filters are lenient per corpus: fields absent from the current corpus schema are ignored, while recognized fields with invalid operator or value types still fail.

## Non-Goals

- Do not modify the graph store, graph service, or graph domain modules.
- Do not add HTTP routes or frontend behavior.
- Do not implement new graph layouts beyond dispatch structure and validation for unsupported graph layouts.
- Do not add cross-corpus filter aliasing or synonym mapping.
- Do not change model provider behavior.
