# Acceptance: RAG Skill Graph Policy

## Acceptance Index

- AC-RSGP-01 [must] [business] Optional graph write whitelists allow valid custom kind and relationship codes; proof: focused skill graph contract passes.
- AC-RSGP-02 [must] [business] Configured graph write whitelists reject out-of-list related graph values; proof: configured-whitelist contract passes.
- AC-RSGP-03 [must] [regression] Production graph repository code has no default skill business whitelist; proof: static repository scan has no hits.
- AC-RSGP-04 [must] [business] Graph skill business fields follow corpus field mapping and domainFields; proof: dynamic mapping contract passes.
- AC-RSGP-05 [must] [api] Edge scalar properties return in graph expansion metadata; proof: edge property contract passes.
- AC-RSGP-06 [must] [regression] existing `skill`, memory, and project-doc RAG write/search paths remain unaffected; proof: full `make be-test` passes.
- AC-RSGP-07 [must] [regression] Tests use fake embedding and rerank runtimes without provider network access; proof: fake runtime contract passes.

## Open Decisions

- None.
