# Backend Task: RAG Pipeline DAG Runtime

## Execution Contract

- phase: tdd
- owner: backend-worker
- branch: `feature/xinlangtao/rag-frontend-release-0.1.0/backend/pipeline-dag-runtime`
- worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-frontend-release-0.1.0/backend/pipeline-dag-runtime`
- base_branch: `feature/xinlangtao/rag-frontend-release-0.1.0/base`

## TDD Contract

Backend worker records RED before production implementation, then GREEN with the same focused RAG test command. Existing full backend failures outside RAG may be recorded as residual environment risk.

## Task Rows

- AC-RPD-01 | phase: tdd | owner: backend-worker | scope: pipeline catalog endpoint and registry | command_ref: CMD-BE-RAG-PIPELINE | status: done.
- AC-RPD-02 | phase: tdd | owner: backend-worker | scope: profile pipeline validation and persistence | command_ref: CMD-BE-RAG-PIPELINE | status: done.
- AC-RPD-03 | phase: tdd | owner: backend-worker | scope: ingest runtime additive stage results | command_ref: CMD-BE-RAG-PIPELINE | status: done.
- AC-RPD-04 | phase: tdd | owner: backend-worker | scope: search runtime additive stage results | command_ref: CMD-BE-RAG-PIPELINE | status: done.
- AC-RPD-05 | phase: tdd | owner: backend-worker | scope: DAG executor artifact contract | command_ref: CMD-BE-RAG-PIPELINE | status: done.
- AC-RPD-08 | phase: verify | owner: backend-worker | scope: backend focused regression | command_ref: CMD-BE-RAG-PIPELINE | status: done.
- AC-RPD-09 | phase: tdd | owner: backend-worker | scope: strict compiler execution plan, artifact availability, and branch/merge levels | command_ref: CMD-BE-PHASE2-RAG | status: done.
- AC-RPD-10 | phase: tdd | owner: backend-worker | scope: ingest context, artifact map, stage executor registry, and generic DAG runtime migration | command_ref: CMD-BE-PHASE2-INGEST | status: done.
- AC-RPD-11 | phase: tdd | owner: backend-worker | scope: retrieval context, artifact map, stage executor registry, and generic DAG runtime migration | command_ref: CMD-BE-PHASE2-SEARCH | status: done.
- AC-RPD-12 | phase: tdd | owner: backend-worker | scope: shared DAG executor branch/merge/failure/stageResults semantics | command_ref: CMD-BE-PHASE2-RAG | status: done.
- AC-RPD-13 | phase: verify | owner: backend-worker | scope: Makefile-first and focused RAG regression after Phase 2 migration | command_ref: CMD-BE-PHASE2-REGRESSION | status: done.
- AC-RPD-14 | phase: tdd | owner: backend-worker | scope: advanced stage catalog and compiler contracts | command_ref: CMD-BE-PHASE3-RAG | status: done.
- AC-RPD-15 | phase: tdd | owner: backend-worker | scope: EXTRACT_TREE and PERSIST_TREE ingest artifact/runtime behavior | command_ref: CMD-BE-PHASE3-INGEST | status: done.
- AC-RPD-16 | phase: tdd | owner: backend-worker | scope: QUESTION_GENERALIZATION and QUERY_REWRITE retrieval artifact/runtime behavior | command_ref: CMD-BE-PHASE3-SEARCH | status: done.
- AC-RPD-17 | phase: tdd | owner: backend-worker | scope: advanced stageResults and artifact handoff proof | command_ref: CMD-BE-PHASE3-SEARCH | status: done.
- AC-RPD-18 | phase: verify | owner: backend-worker | scope: Makefile-first and focused RAG regression after Phase 3 advanced stages | command_ref: CMD-BE-PHASE3-REGRESSION | status: done.
- AC-RPD-19 | phase: tdd | owner: backend-worker | scope: trace-enabled search persists safe stageResults summaries and trace-disabled search writes no diagnostic row | command_ref: CMD-BE-PHASE4-TRACE | status: done.
- AC-RPD-21 | phase: verify | owner: backend-worker | scope: Makefile-first and focused RAG regression after Phase 4 trace enhancements | command_ref: CMD-BE-PHASE4-REGRESSION | status: done.

## Worktree Notes

- Existing backend worker branch mirrors selected git skill branch tree and is retained as prior delivery evidence.
- Phase 2 runtime worker uses `backend/dag-runtime-phase2` so stricter runtime migration does not overwrite prior retained evidence.
- Phase 3 advanced-stage worker uses `backend/advanced-stages-phase3` so advanced stage delivery stays isolated from prior Phase 2 runtime evidence.
- Phase 4 trace worker uses `backend/trace-stage-results-phase4` so diagnostics observability changes stay isolated from previous runtime evidence.
- Worker writes backend code, backend tests, backend evidence report, and assigned orchestration fragments only.

## Integration Notes

- Lead merges backend branch into base after review and reruns affected verification.

## Open Decisions

- None.
