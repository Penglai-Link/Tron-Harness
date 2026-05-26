# Frontend Task: Right Context Sidebar

## Execution Contract

- feature_slug: right-context-sidebar
- status: waiting_human_decision
- execution_mode: hybrid
- canonical_report_target: reports/frontend-result.md

## TDD Contract

Behavior-changing implementation should run in bounded AC slices after execution approval. Default dispatch is hybrid because the feature touches the shared shell and multiple page groups. Adjacent AC rows may be grouped only when write scopes and verification commands remain coherent.

## Task Rows

- AC-RCS-01 | phase: tdd | owner: frontend | scope: `tron-frontend/src/shared/layout/Shell.tsx`, `tron-frontend/src/store/appShellStore.ts`, `tron-frontend/src/i18n/locales/en-US/shell.ts` desktop right sidebar state and controls | command_ref: CMD-FE-CHECK | status: waiting_human_decision.
- AC-RCS-02 | phase: tdd | owner: frontend | scope: `tron-frontend/src/shared/layout/Shell.tsx` mobile right drawer open, close, backdrop, Escape, and route reset behavior | command_ref: CMD-FE-CHECK | status: waiting_human_decision.
- AC-RCS-03 | phase: tdd | owner: frontend | scope: current shell right-panel routes, no-panel console routes, and settings route exclusion | command_ref: CMD-FE-CHECK | status: waiting_human_decision.
- AC-RCS-04 | phase: tdd | owner: frontend | scope: `tron-frontend/src/domain/knowledge-graph/pages/*`, `tron-frontend/src/domain/memory/pages/*` local inspector migration | command_ref: CMD-FE-CHECK | status: waiting_human_decision.
- AC-RCS-05 | phase: tdd | owner: frontend | scope: task detail, automation detail, app detail, prompts, variables, skills, installed skills, and knowledge base local inspector migration | command_ref: CMD-FE-CHECK | status: waiting_human_decision.

## Worktree Notes

- A Lite Spec feature branch or multi-agent base branch must be created before behavior-changing edits.
- No implementation has started.

## Integration Notes

- Shell foundation ACs should land before page migration ACs.
- Page migrations should preserve selected-item state in each page and move only contextual panel placement into the shell.

## Open Decisions

- Execution approval and dispatch mode are pending.
