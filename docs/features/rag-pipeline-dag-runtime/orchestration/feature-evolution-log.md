# Feature Evolution Log: RAG Pipeline DAG Runtime

## Summary

- feature_slug: rag-pipeline-dag-runtime
- status: complete_through_phase4_trace_workbench

## Change History

- change_id: 1
- trigger: user_request
- change_type: feature_model_expansion
- gate_reopened: initial_delivery
- diff_summary: Introduce Pipeline DAG profile model across management, runtime, and frontend surfaces.
- downstream_impact: spec, acceptance, test plan, tasks, reports, worktree plan.
- old_evidence_status: not_applicable
- user_approval: approved_by_request

- change_id: 2
- trigger: completion_audit
- change_type: acceptance_expansion
- gate_reopened: Phase 4D acceptance and Phase 6 execution
- diff_summary: Previous evidence proved catalog/profile/Workbench surfaces but did not prove Phase 2 runtime migration. Reopen runtime-specific ACs for ingest/retrieval contexts, artifact store map, compiler, and generic DAG executor.
- downstream_impact: acceptance, test plan, backend task, worktree plan, reports.
- old_evidence_status: retained_for_phase1_phase4_superseded_for_phase2_runtime
- user_approval: approved_by_goal_continuation

- change_id: 3
- trigger: user_request
- change_type: requirement_expansion
- gate_reopened: Phase 4D acceptance and Phase 6 execution
- diff_summary: Add Phase 3 advanced pipeline stages for ingest tree extraction/persistence and retrieval query generalization/rewrite while preserving Phase 2 DAG/artifact runtime semantics.
- downstream_impact: acceptance, test plan, backend task, worktree plan, reports.
- old_evidence_status: retained
- user_approval: approved_by_goal_continuation

- change_id: 4
- trigger: user_request
- change_type: observability_and_interaction_expansion
- gate_reopened: Phase 4D acceptance and Phase 6 execution
- diff_summary: Add Phase 4 trace and Workbench closure: search trace payload must include safe stageResults summaries, and Workbench must prove detailed stage result rendering plus legacy fallback behavior.
- downstream_impact: acceptance, test plan, backend task, frontend task, worktree plan, reports, final evidence.
- old_evidence_status: retained_for_phase1_phase3_superseded_for_phase4_trace_and_workbench_closure
- user_approval: approved_by_goal_continuation

## Downstream Artifact Migration

- New feature workspace created because scope spans ingest, retrieval, management, frontend, and workbench.
- AC-RPD-01 through AC-RPD-08 evidence remains retained for catalog/profile/Workbench compatibility where still true.
- AC-RPD-05 evidence was superseded by the stricter Phase 2 runtime proof; AC-RPD-09 through AC-RPD-13 now carry current DAG runtime evidence.
- AC-RPD-14 through AC-RPD-18 evidence remains retained for Phase 3 advanced-stage delivery.
- AC-RPD-07 evidence is retained as initial Workbench coverage but superseded by AC-RPD-20 for Phase 4 detailed trace/workbench closure.

## Residual Risks

- None for Phase 4. Full backend Makefile still has known non-RAG auth/MCP environment failures, with focused RAG regression passing.
