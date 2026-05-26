# Frontend Task: Right Context Sidebar

## Execution Contract

- feature_slug: right-context-sidebar
- status: in_progress
- execution_mode: hybrid
- canonical_report_target: reports/frontend-result.md

## TDD Contract

Behavior-changing implementation should run in bounded AC slices after execution approval. Default dispatch is hybrid because the feature touches the shared shell and multiple page groups. Adjacent AC rows may be grouped only when write scopes and verification commands remain coherent.

## Task Rows

- AC-RCS-01 | phase: tdd | owner: frontend | scope: `tron-frontend/src/shared/layout/Shell.tsx`, `tron-frontend/src/store/appShellStore.ts`, `tron-frontend/src/i18n/locales/en-US/shell.ts` desktop right sidebar state and controls | command_ref: CMD-FE-CHECK | status: done.
- AC-RCS-02 | phase: tdd | owner: frontend | scope: `tron-frontend/src/shared/layout/Shell.tsx` mobile right drawer open, close, backdrop, Escape, and route reset behavior | command_ref: CMD-FE-CHECK | status: done.
- AC-RCS-03 | phase: tdd | owner: frontend | scope: current shell right-panel routes, no-panel console routes, and settings route exclusion | command_ref: CMD-FE-CHECK | status: done.
- AC-RCS-04 | phase: tdd | owner: frontend | scope: `tron-frontend/src/domain/knowledge-graph/pages/*`, `tron-frontend/src/domain/memory/pages/*` local inspector migration | command_ref: CMD-FE-CHECK | status: done.
- AC-RCS-05 | phase: tdd | owner: frontend | scope: task detail, automation detail, app detail, prompts, variables, skills, installed skills, and knowledge base local inspector migration | command_ref: CMD-FE-CHECK | status: in_progress.

## Worktree Notes

- A Lite Spec feature branch or multi-agent base branch must be created before behavior-changing edits.
- AC-RCS-01 through AC-RCS-03 integrated from `frontend/shell-foundation` at merge commit `2b20064`.
- AC-RCS-04 dispatched to `frontend/graph-memory-migration` with Codex runtime `019e6250-af5a-7e03-adc1-ad1680251253`.
- AC-RCS-04 integrated from `frontend/graph-memory-migration` at merge commit `3b76629`.
- AC-RCS-05 dispatched to `frontend/management-migration` with Codex runtime `019e625d-dc11-79d2-a264-033738da4c71`.

## Integration Notes

- Shell foundation ACs landed before page migration ACs.
- Page migrations should preserve selected-item state in each page and move only contextual panel placement into the shell.
- AC-RCS-05 dispatch waits for AC-RCS-04 integration so management migration can reuse the graph and memory migration pattern.

## Open Decisions

- None.
