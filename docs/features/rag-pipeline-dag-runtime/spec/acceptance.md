# Acceptance: RAG Pipeline DAG Runtime

## Acceptance Index

- AC-RPD-01 [must] [api] Pipeline catalog exposes stage definitions, strategies, artifact contracts, join policies, and failure policies; proof: catalog contract tests pass.
- AC-RPD-02 [must] [api] Profile management accepts Pipeline DAG and rejects invalid nodes, edges, cycles, stages, strategies, and config; proof: profile verification tests pass.
- AC-RPD-03 [must] [business] Ingest request remains unchanged and response preserves old fields while adding stage results; proof: ingest compatibility tests pass.
- AC-RPD-04 [must] [business] Search request remains unchanged and response preserves old fields while adding stage results; proof: search compatibility tests pass.
- AC-RPD-05 [must] [business] DAG execution supports branch and merge while stages own artifact input and output keys; proof: executor unit tests pass.
- AC-RPD-06 [must] [interaction] Profile forms render catalog-backed pipeline nodes, edges, config, preview, and verification; proof: profile form browser tests pass.
- AC-RPD-07 [must] [interaction] Workbench displays stage results when present and keeps legacy summaries otherwise; proof: workbench browser tests pass.
- AC-RPD-08 [must] [regression] Existing runtime ingest and search examples remain compatible after the Pipeline DAG upgrade; proof: focused regression checks pass.
- AC-RPD-09 [must] [business] Pipeline compiler produces executable DAG levels from stage-owned artifact contracts and rejects cycles, missing inputs, repeatability violations, and strategy/stage mismatches; proof: compiler RED/GREEN tests pass.
- AC-RPD-10 [must] [business] Ingest execution runs through the generic DAG runtime while preserving the ingest API contract; proof: ingest runtime RED/GREEN tests pass.
- AC-RPD-11 [must] [business] Retrieval execution runs through the generic DAG runtime while preserving the search API contract; proof: retrieval runtime RED/GREEN tests pass.
- AC-RPD-12 [must] [business] DAG execution supports branch, parallel-ready sets, merge join policies, failure policies, and stageResults with input/output artifact keys for both ingest and retrieval pipelines; proof: DAG executor unit tests pass.
- AC-RPD-13 [must] [regression] Existing RAG ingest/search focused API and contract suites remain compatible after Phase 2 runtime migration; proof: focused backend regression commands pass or known non-RAG Makefile failures are isolated.
- AC-RPD-14 [must] [api] Pipeline catalog and compiler expose Phase 3 advanced stages with stage-owned artifact contracts; proof: catalog/compiler tests pass.
- AC-RPD-15 [must] [business] Ingest pipelines can extract and persist tree graph artifacts without changing ingest requests; proof: ingest contract tests pass.
- AC-RPD-16 [must] [business] Retrieval pipelines can generalize and rewrite queries before route execution without changing search requests; proof: search contract tests pass.
- AC-RPD-17 [must] [business] Advanced stageResults reflect actual executed advanced nodes and artifact handoffs; proof: runtime stage result tests pass.
- AC-RPD-18 [must] [regression] Existing RAG ingest/search suites remain compatible after Phase 3 advanced stages; proof: focused backend regression commands pass or known non-RAG Makefile failures are isolated.
- AC-RPD-19 [must] [business] Search trace persistence records safe stageResults summaries for executed retrieval pipeline nodes when trace is enabled and writes no trace row when trace is disabled; proof: diagnostics trace tests pass.
- AC-RPD-20 [must] [interaction] Workbench renders ingest and search stageResults detail rows from runtime responses and keeps legacy fallback display when stageResults are absent; proof: Workbench browser tests pass.
- AC-RPD-21 [must] [regression] Existing RAG ingest/search request contracts and focused suites remain compatible after Phase 4 trace and Workbench enhancements; proof: Makefile-first plus focused regression commands pass or known non-RAG failures are isolated.

## Open Decisions

- None.
