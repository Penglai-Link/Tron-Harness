# Acceptance: RAG Storage Backend Fusion

- AC-RSBF-01 [must] [backend] RAG corpus storage bindings expose explicit backend values `SQLITE` or `GRAPH`; repository dispatch uses backend first and preserves existing SQLite and graph skill behavior.
- AC-RSBF-02 [must] [config] Seeded retrieval profiles include a three-corpus fusion profile for `skill-graph`, `memory-card`, and `project-doc` using RRF and graph expansion.
- AC-RSBF-03 [must] [backend] Request search filters are applied only to corpora whose filter schema declares the field; unknown request filter fields for a route are ignored, while invalid values for known fields still fail.
- AC-RSBF-04 [must] [backend] Graph-backed vector route queries carrying a query embedding build graph prepared search with semantic projection and resolved embedding model code, without changing graph store modules.
- AC-RSBF-05 [must] [quality] Focused RAG backend tests prove storage backend parsing, three-corpus fusion, lenient filters, graph semantic projection, and existing SQLite search regressions.
