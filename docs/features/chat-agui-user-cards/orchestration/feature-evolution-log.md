# Feature Evolution Log: Chat AG-UI User Cards

## Evolution Summary

### 2026-06-04 segment split refinement

- change_request: Split user-facing Thinking cards by reasoning segment instead of aggregating every reasoning loop in a run into one card.
- classification: behavior refinement
- downstream_impact: frontend-only aggregation logic, mock AG-UI stream data, and Playwright browser proof change; backend event protocol stays unchanged.
- stale_artifacts: `spec/feature-spec.md`, `spec/acceptance.md`, `spec/test-plan.md`, `tasks/frontend-task.md`, `orchestration/worktree-plan.md`, `orchestration/context-index.md`, and `reports/test-report.md` previously described one Thinking card per run and required refresh.
- affected_validation_gates: spec, tasks, reports, browser verify, frontend build/typecheck
- evidence_migration: replace prior AC-AGUI-02 evidence that asserted one grouped Thinking card per run with new evidence that asserts one Thinking card per reasoning segment within the latest run.

### 2026-06-04 Codex-style work rail refinement

- change_request: Reframe `/chats` user-facing AG-UI rendering into a compact work rail so Thinking segments stay ahead of the assistant answer, collapsed summaries read like short work summaries, and nested tool activity no longer looks like card-within-card chat bubbles.
- classification: behavior refinement
- downstream_impact: frontend-only transcript presentation and Playwright browser proof change; backend event protocol and AG-UI debug rail stay unchanged.
- stale_artifacts: `spec/feature-spec.md`, `spec/acceptance.md`, `spec/test-plan.md`, `tasks/frontend-task.md`, `orchestration/context-index.md`, and `reports/test-report.md` required refresh because they still described nested tool cards rather than compact tool steps and did not require work-rail ordering ahead of assistant output.
- affected_validation_gates: spec, tasks, reports, browser verify, frontend build/typecheck
- evidence_migration: replace prior AC-AGUI-03 evidence that asserted nested tool cards with evidence that asserts compact nested tool steps and work-rail-first ordering.

### 2026-06-05 session replay refinement

- change_request: Keep recent completed AG-UI work rails visible per chat session after page refresh and chat switching.
- classification: behavior refinement with backend replay persistence fix
- downstream_impact: frontend transcript aggregation now renders recent per-turn work rails instead of only the latest run; backend replay records HTTP-layer system-agent live protocol events and merges SQLite replay with in-memory sink events.
- stale_artifacts: `spec/acceptance.md`, `spec/test-plan.md`, `orchestration/worktree-plan.md`, `orchestration/context-index.md`, and `reports/test-report.md` needed refresh for AC-AGUI-07 and real Chromium evidence.
- affected_validation_gates: backend check, frontend check, real browser refresh/switch verification
- evidence_migration: add session-level replay evidence for a real two-turn chat session showing two work rails before refresh, after refresh, and after switching away and back.

## Scope Decision

- backend_change_required: yes
- affected_ac_refs: AC-AGUI-02, AC-AGUI-03, AC-AGUI-07
- unchanged_ac_refs: AC-AGUI-01, AC-AGUI-04, AC-AGUI-05, AC-AGUI-06
