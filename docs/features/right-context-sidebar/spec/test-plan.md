# Test Plan: Right Context Sidebar

## Test Plan Decision

- status: approved
- approved_by: clouds3n
- approved_at: 2026-05-26

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-RCS-01 | layer: browser | case: desktop shell right sidebar expands, collapses out of layout, restores main width, and persists collapsed preference | red: the right panel remains a static column, collapses to a content rail, leaves main content constrained, or shows controls when no right content exists | green: Playwright proves a current shell right-panel route shows the expanded sidebar, the collapse control removes the sidebar content from the grid, a right-edge reopen control appears, reopening restores the panel, and local storage contains `rightSidebarCollapsed` under `tron-app-shell` | interaction: open Shared Shell Preview or Chat -> collapse right sidebar -> verify content disappears and main area widens -> reopen -> verify content returns | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER.
- AC-RCS-02 | layer: browser | case: mobile shell right drawer opens and closes predictably without stacking below content | red: right content is hidden, appears below the main content, cannot be opened, or remains open after backdrop, Escape, or route change | green: Playwright mobile viewport proves the drawer opens from shell control, renders right content over a backdrop, closes by close button, backdrop, Escape, and closes after navigation | interaction: set mobile viewport -> open a route with right content -> open drawer -> close by each supported path -> navigate away -> verify drawer is closed | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER.
- AC-RCS-03 | layer: browser | case: existing shell right-panel pages and no-panel/settings pages keep correct right-sidebar presence | red: Chat, Observability, or Shared Shell Preview loses right content, Observability closed inspector leaves a stale toggle, no-panel pages show a toggle, or settings routes show right sidebar controls | green: Playwright proves Chat and preview retain right content, Observability can remove it without stale controls, a no-panel console page has no right-sidebar controls, and settings pages have no right sidebar | interaction: visit Chat, Observability, Shared Shell Preview, Dashboard, and Settings -> assert sidebar presence or absence by page contract | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER.
- AC-RCS-04 | layer: browser | case: graph and memory local inspectors move into the shell right sidebar and keep selected-item behavior | red: graph or memory pages keep local right grid inspectors, lose selected entity/fragment details, or cannot reopen contextual content after selection | green: Playwright proves graph index/domain and memory index/details render contextual details in the shell right sidebar, selection updates the sidebar content, and closing/reopening follows shell behavior | interaction: open graph and memory pages -> select an entity or fragment -> verify shell sidebar content updates -> collapse/reopen -> verify selected context remains | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER.
- AC-RCS-05 | layer: browser | case: management and detail local inspectors move into the shell right sidebar and keep page-specific behavior | red: task, automation, app, prompt, variable, skill, installed skill, or knowledge-base pages keep local right inspector grids, lose contextual actions, or break selection/close/tab behavior | green: Playwright proves representative migrated pages render contextual panels through the shell sidebar, preserve selected content, and keep relevant actions/tabs working | interaction: open representative routes from each page group -> select or inspect current item -> verify shell sidebar content and actions -> collapse/reopen -> verify context remains | command_ref: CMD-FE-RIGHT-SIDEBAR-BROWSER.

## Command Index

- CMD-FE-INSTALL | command: rtk make fe-install | cost: medium | when: verify workspace prerequisites | rerun: only when frontend dependencies are missing in the current worktree | expect: frontend dependencies are available.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: green/verify/final | rerun: affected frontend changes | expect: TypeScript and production build pass.
- CMD-FE-RIGHT-SIDEBAR-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line | cost: slow/browser | when: red/green/verify | rerun: affected shell or migrated page changes | expect: right sidebar desktop, mobile, existing right-panel pages, settings exclusion, and migrated inspector flows pass with concrete browser evidence.
- CMD-FE-SHELL-REGRESSION | command: rtk npm --prefix tron-frontend exec playwright -- test shell-theme-switcher.spec.ts shared-ui-feedback.spec.ts --reporter=line | cost: slow/browser | when: verify/final | rerun: shell layout or shared preview changes | expect: existing shell navigation, shell persistence, and shared shell preview behavior remain valid.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/right-context-sidebar --phase final | cost: medium | when: final | rerun: after doc, task, report, or orchestration edits | expect: right-context-sidebar feature docs validate end to end.

## Evidence Rules

- Every `AC-RCS-*` row must have final evidence in reports.
- Browser evidence must include clicks, inputs when applicable, network assertions, visible feedback, and artifact paths.
- Migration evidence must name the page group and prove the contextual panel is shell-owned rather than a local grid column.
- Review may reuse fresh slow/browser evidence unless relevant shell or page migration paths change again.

## Open Decisions

- None.
