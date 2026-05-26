# AC-RCS-04 Frontend Graph + Memory Migration Evidence

## Changed Paths

- `tron-frontend/src/domain/knowledge-graph/pages/KnowledgeGraphPage.tsx`
- `tron-frontend/src/domain/knowledge-graph/pages/KnowledgeGraphDomainPage.tsx`
- `tron-frontend/src/domain/memory/pages/MemoryPage.tsx`
- `tron-frontend/src/domain/memory/pages/MemoryDetailsPage.tsx`
- `tron-frontend/tests/right-context-sidebar.spec.ts`
- `docs/features/right-context-sidebar/reports/agents/AC-RCS-04-frontend-graph-memory-migration.md`

## Review note

- Lead review identified a gap: `KnowledgeGraphDomainPage.tsx` was still using a legacy local `GraphInspector` with a 3-column grid. This was fixed by reusing the shell-integrated `GraphPage` and routing `domainFiltering` through the shared shell-bound inspector path.

## RED Evidence

- Temporary base evidence worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/red-evidence/frontend-rcs-04-base`
- Base source: detached worktree from `feature/clouds3n/right-context-sidebar/base` at commit `0bb4e2ed1fa7c22bb5ec86f965ee5d8fe5130fa2`.
- Setup: copied the updated `tron-frontend/tests/right-context-sidebar.spec.ts` (including the new `AC-RCS-04` test) into the detached base worktree.
- RED command: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
- RED result: fail on `/knowledge-graph` as expected because no shell right sidebar was rendered for graph pages.
  - `expect(rightSidebar(page)).toBeVisible()` failed at `AC-RCS-04` before any graph selection assertion executed.
- Temporary base worktree was then removed: `rtk git worktree remove --force /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/right-context-sidebar/red-evidence/frontend-rcs-04-base`

## GREEN Evidence

### Graph pages

- Visited `/knowledge-graph` (desktop, 1440x900). `tron-right-sidebar` rendered with shell heading `Node Inspector`, showing selected entity `Customer_0192`.
- Selected entity switch (`Order_TXN_55`) updated `tron-right-sidebar` content.
- Collapsing and reopening the right sidebar preserved selected graph context.
- Visiting `/knowledge-graph/domain-filtering` confirmed domain-filtering controls remained present and inspector context continued to render in shell.
- `KnowledgeGraphDomainPage` no longer renders a third local inspector column; the AC-RCS-04 test asserted the domain graph container has a single `aside` column and shell-owned `Node Inspector` updates with entity selection (`Customer_0192` -> `Order_TXN_56`).

### Memory pages

- Visited `/memory` and `/memory/details`. Shell sidebar rendered memory inspector context instead of local right grid inspector.
- Default selected fragment (`mem-1`) content was visible on `/memory` in shell.
- Selecting `System default tone ...` and `Project Alpha ...` rows updated shell context text in-place.
- Collapsing and reopening the right sidebar preserved the selected fragment context across reopen.

## AC-RCS-04

- Graph local inspector (`xl:grid-cols-[300px_minmax(520px,1fr)_320px]`) and memory local right inspector grid (`xl:grid-cols-[minmax(0,1fr)_400px]`) were removed from these pages.
- Graph and memory inspectors now render through `useConsoleShellConfig({ rightPanel })` and `InspectorPanel`.
- Selected-entity / selected-fragment behavior is preserved and shell collapse/reopen preserves selected context.
- Domain filtering (`domain-filtering` page) and zoom controls are unchanged.

## Commands

- PASS: `rtk make fe-install`
- PASS: `rtk make fe-install`
  - Base worktree install for RED evidence.
- FAIL: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - RED worker-branch base reference: 3 passed, AC-RCS-04 failed with `right sidebar not visible` on `/knowledge-graph`.
- PASS: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - GREEN on worker branch: all 4 tests passed including AC-RCS-04.
- PASS: `rtk make fe-check`
  - Frontend typecheck and production build pass.

## Blockers

- No blockers encountered for AC-RCS-04 implementation and verification within the assigned scope.
