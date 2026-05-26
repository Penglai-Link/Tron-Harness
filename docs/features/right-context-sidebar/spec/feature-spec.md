# Feature Spec: Right Context Sidebar

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved

## Feature Identity And Context

- feature_slug: right-context-sidebar
- feature_name: Right Context Sidebar
- status: approved
- owner: clouds3n
- source_request: Add a conditional right sidebar that defaults collapsed, behaves like the left sidebar after opening, collapses out of layout on desktop, uses a mobile drawer, excludes global configuration pages, and migrates existing local right inspectors into the shared shell contract.
- main_agent: Codex lead agent
- dependency_features: `shell-theme-switcher`, `shared-ui-feedback`
- dependency_contracts: existing `TronShell`, `ConsoleShellConfig`, `useConsoleShellConfig`, Zustand `appShellStore`, shell i18n resources, existing page modules with local contextual right panels

## Requirement

Create a shell-level right context sidebar for console pages. A page should provide contextual content through the existing shell configuration contract, while the shell owns placement, collapse behavior, mobile drawer behavior, and accessibility controls.

The sidebar appears only when a console page provides contextual content. Desktop starts collapsed with only a right-edge open control, expands on demand, and collapse removes the right sidebar from layout. Mobile presents the same content as a right overlay drawer. Settings and global configuration pages do not participate.

Existing pages that already render a local right-side contextual inspector should migrate that inspector into the shell-level contract. The migration must preserve page behavior and must not add new domain content.

## Product Source Slices

- approved_brainstorm: `docs/brainstorm/right-context-sidebar/2026-05-26-10-12-25.md`
- approved_prd: `docs/brainstorm/right-context-sidebar/prd.typ`
- selected_feature: shell-level right context sidebar with migrated local contextual inspectors

## Scope Boundary

This feature is frontend-only. It changes the shared console shell, shell state store, shell i18n labels, existing shell right-panel pages, and existing page modules that currently own local contextual right inspectors.

## Assumptions

- The existing `rightPanel` shell contract is the correct extension point for contextual right-side content.
- Local right inspectors that explain a selected row, entity, task, skill, variable, app, or knowledge item are contextual content and should move to the shell-level sidebar.
- Settings pages remain on `SettingsShell` and do not need a right sidebar.
- Desktop right sidebar state should reset to collapsed when entering a route with contextual right content; expanded state is not a durable preference.
- Mobile drawer open state should be local shell state and should reset on route changes.
- No backend state or API contract is required.

## In Scope

- Add right sidebar collapsed state to `appShellStore` without persisting the expanded preference.
- Update `TronShell` desktop layout so the right sidebar expands at the existing right-panel width and collapses fully out of layout.
- Render a right-edge open control when a page has right content and the desktop right sidebar is in its default collapsed state.
- Render right content as a mobile overlay drawer with backdrop, close button, Escape close, and route-change reset.
- Add shell i18n labels for opening, collapsing, and closing the right sidebar.
- Preserve current shell right-panel users: Chat reasoning, Chat execution, Observability, and Shared Shell Preview.
- Migrate existing local contextual right inspectors from graph, task detail, automation detail, memory, prompts, variables, skills, installed skills, knowledge base, and app detail page groups.
- Verify no right sidebar appears in settings/global configuration pages.

## Out Of Scope

- Backend API changes.
- Database schema or persistence changes.
- Route metadata registry for right sidebar content.
- New business content for pages that do not already provide shell right content or a local contextual right inspector.
- Full redesign of page layouts unrelated to contextual right panels.
- New animation framework.
- Full `zh-CN` localization, because current frontend resources expose only `en-US`.

## Constraints

- Always use `Makefile` commands first for validation entry points.
- Shell changes must not break left sidebar collapse, mobile left navigation, workspace selector, utility popovers, agent launcher, or user menu behavior.
- Desktop collapse must recover main content width; it must not leave a content rail.
- Mobile right drawer must not conflict with the mobile left navigation drawer.
- Existing inspector close/reopen behavior should remain understandable after migration.
- Existing selected-item state must remain page-owned.

## Feature Model

Bounded context:

- Shared console shell contextual side panel.

Entities:

- `RightSidebarContent`: page-owned React content passed through shell configuration.
- `RightSidebarCollapsed`: desktop shell state that defaults to collapsed on route entry and is not persisted as an expanded preference.
- `RightSidebarDrawerOpen`: shell-local mobile state.
- `LocalInspector`: existing page-owned right contextual panel before migration.

Policies:

- Pages own what appears inside the sidebar.
- The shell owns where and how the sidebar appears.
- A page without right content produces no right sidebar controls.
- Collapsed desktop state applies only while right content exists and resets to collapsed on route entry.
- Mobile drawer state resets on route change.
- Local inspector migrations keep domain interaction state in the page and only move presentation placement into the shell.

## Page And Module Surface

Shell foundation:

- `tron-frontend/src/shared/layout/Shell.tsx`
- `tron-frontend/src/store/appShellStore.ts`
- `tron-frontend/src/i18n/locales/en-US/shell.ts`
- `tron-frontend/src/app/App.tsx`

Current shell right-panel users:

- `tron-frontend/src/domain/chat/pages/ChatSessionReasoningPage.tsx`
- `tron-frontend/src/domain/chat/pages/ChatSessionExecutionPage.tsx`
- `tron-frontend/src/domain/observability/pages/ObservabilityPage.tsx`
- `tron-frontend/src/domain/shared-shell/pages/SharedShellPreviewPage.tsx`

Implemented local inspector migrations:

- `tron-frontend/src/domain/knowledge-graph/pages/KnowledgeGraphPage.tsx`
- `tron-frontend/src/domain/knowledge-graph/pages/KnowledgeGraphDomainPage.tsx`
- `tron-frontend/src/domain/task/pages/TaskDetailsPage.tsx`
- `tron-frontend/src/domain/automation/pages/AutomationDetailsPage.tsx`
- `tron-frontend/src/domain/memory/pages/MemoryPage.tsx`
- `tron-frontend/src/domain/memory/pages/MemoryDetailsPage.tsx`
- `tron-frontend/src/domain/prompt/pages/PromptsPage.tsx`
- `tron-frontend/src/domain/variable/pages/VariableManagementPage.tsx`
- `tron-frontend/src/domain/variable/pages/VariableDetailsPage.tsx`
- `tron-frontend/src/domain/skill/pages/SkillsPage.tsx`
- `tron-frontend/src/domain/skill/pages/InstalledSkillsPage.tsx`
- `tron-frontend/src/domain/knowledge-base/pages/KnowledgeBasePage.tsx`
- `tron-frontend/src/domain/app/pages/AppDetailsPage.tsx`

## Repository Findings

- `TronShell` accepts `rightPanel` and owns desktop default-collapsed placement, centered boundary collapse/reopen controls, and mobile drawer placement.
- `useConsoleShellConfig` lets pages provide dynamic shell content without static route metadata.
- `appShellStore` persists left sidebar collapsed state under `tron-app-shell`, while right sidebar expanded state remains route-entry local and defaults collapsed.
- Mobile left navigation already uses overlay/backdrop behavior that can guide the right drawer implementation.
- The implemented page inventory uses shell `rightPanel` for existing shell users, graph/memory inspectors, and representative management/detail inspectors.
- Settings and global configuration routes remain excluded from the right context sidebar contract.
- `VariableDetailsPage` delegates to `VariableManagementPage`; both routes share the shell sidebar placement contract.
- The frontend i18n folder currently exposes only `en-US`, so new shell labels should follow the existing resource shape.

## Feature Decomposition

- Desktop shell state and layout: foundational shared shell behavior.
- Mobile right drawer: responsive presentation for the same content.
- Existing shell right-panel regression: proof that current shell users still work.
- Local inspector migration: grouped page migrations after the shell contract is stable.

One `docs/features/right-context-sidebar` workspace is appropriate because all slices share the same shell contract and serve one user-facing capability.

## Open Decisions

- None.
