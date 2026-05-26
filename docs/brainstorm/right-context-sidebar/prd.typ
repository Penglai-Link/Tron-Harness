= PRD: Right Context Sidebar

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron console shell right context sidebar.
- Problem: pages can provide `rightPanel` content today, but the panel is a fixed desktop column and does not behave like a first-class collapsible shell surface.
- Target users: workspace operators, platform admins, AI workflow maintainers, and engineering leads using context-heavy console pages.
- Outcome: console pages with contextual content get a consistent right sidebar that can disappear from desktop layout when collapsed and appears as a drawer on mobile.

== Goals
- Turn the existing shell `rightPanel` slot into a consistent conditional right context sidebar.
- Let pages keep ownership of sidebar content while the shell owns layout, collapse, drawer, and accessibility behavior.
- Preserve full-width main content when the right sidebar is collapsed.
- Keep mobile access to sidebar content through a right-side drawer.
- Exclude settings and global configuration pages from this sidebar system.
- Regress existing `rightPanel` users and migrate existing local right inspectors into the same shell-level sidebar contract.

== Non-Goals
- New right sidebar content for pages that do not already provide `rightPanel` or an existing local right inspector.
- Any backend API, data model, or permission change.
- A route-metadata-driven right panel registry.
- New animation framework or broad shell redesign.

== Scope
- In scope: console shell right sidebar state, desktop collapse behavior, mobile drawer behavior, shell i18n labels, regression coverage for current shell `rightPanel` pages, and migration of existing page-owned local right inspectors.
- Out of scope: settings shell, global configuration routes, backend behavior, and new domain content.

== Key Requirements
- The sidebar appears only when the active console page provides `rightPanel`.
- Pages continue to set sidebar content through `useConsoleShellConfig({ rightPanel })`.
- When collapsed on desktop, the sidebar content leaves the layout and main content regains the available width.
- The collapsed desktop state leaves only a right-edge open control with a clear accessible label.
- When expanded on desktop, the sidebar keeps the existing 360px width and provides a visible collapse control.
- On mobile, sidebar content opens as a right-side overlay drawer with backdrop, close button, `Escape` close, and route-change reset.
- If a page removes `rightPanel`, the shell renders no sidebar and no stale right-edge toggle.
- Existing page-owned local right inspectors migrate to `useConsoleShellConfig({ rightPanel })` when the panel is contextual content.
- Migration preserves existing selected-row, selected-entity, selected-task, tab, and close behavior.
- `SettingsShell` and global configuration pages do not show this sidebar.

== Shared Requirements
- Shell layout state: shared because left sidebar, right sidebar, floating panels, and workspace selector all coexist in one persistent shell; consumers: Feature 1, Feature 2; decision: build first.
- Page shell config contract: shared because pages already inject search, actions, content class, and `rightPanel`; consumers: Feature 1, Feature 3; decision: keep local to shell and avoid route metadata for now.
- Responsive overlay behavior: shared because mobile navigation and right drawer both need predictable backdrop, escape, and route-change cleanup; consumers: Feature 2; decision: build first inside shell.
- Shell accessibility labels: shared because sidebar controls must be discoverable and localized; consumers: Feature 1, Feature 2, Feature 4; decision: build with the shell changes.
- Inspector migration pattern: shared because multiple pages must move local right panels into the shell without changing domain behavior; consumers: Feature 4; decision: build after shell foundation and drawer behavior are stable.

== Feature Inventory
1. Desktop Right Sidebar State: goal make the current right panel a collapsible desktop shell surface; in scope persisted `rightSidebarCollapsed`, expanded 360px layout, collapsed full-width main layout, right-edge open control, and expanded collapse control; out of scope page content changes; depends on shared requirement: shell layout state.
2. Mobile Right Drawer: goal keep contextual sidebar content accessible on smaller screens without pushing it below main content; in scope right-side drawer, backdrop, close button, `Escape`, route-change reset, and no drawer when `rightPanel` is absent; out of scope advanced focus trap library or new animation framework; depends on Feature 1 and shared requirement: responsive overlay behavior.
3. Existing Shell Panel Regression: goal preserve current pages that already use shell `rightPanel`; in scope Chat, Observability, Shared Shell Preview, no-sidebar pages, and settings pages; local inspector migration is handled by Feature 4; depends on Feature 1 and Feature 2.
4. Local Inspector Migration: goal move existing page-owned right inspectors into the shell-level sidebar contract; in scope graph, task detail, automation detail, memory, prompt, variables, skills, knowledge base, and app detail pages where the right panel is contextual; out of scope new domain content or backend changes; depends on Feature 1, Feature 2, and shared requirement: inspector migration pattern.

== Dependency Map
- Feature 1 depends on shared requirement: shell layout state.
- Feature 2 depends on Feature 1.
- Feature 2 depends on shared requirement: responsive overlay behavior.
- Feature 3 depends on Feature 1 and Feature 2.
- Feature 3 depends on shared requirement: page shell config contract.
- Feature 4 depends on Feature 1 and Feature 2.
- Feature 4 depends on shared requirement: inspector migration pattern.

#mermaid(
  "flowchart LR
    SR1[\"Shared Requirement: Shell Layout State\"] --> F1[\"Feature 1: Desktop Right Sidebar State\"]
    F1 --> F2[\"Feature 2: Mobile Right Drawer\"]
    SR2[\"Shared Requirement: Responsive Overlay Behavior\"] --> F2
    SR3[\"Shared Requirement: Page Shell Config Contract\"] --> F3[\"Feature 3: Existing Shell Panel Regression\"]
    F1 --> F3
    F2 --> F3
    F1 --> F4[\"Feature 4: Local Inspector Migration\"]
    F2 --> F4
    SR4[\"Shared Requirement: Inspector Migration Pattern\"] --> F4"
)

== Recommended Implementation Order
1. Desktop right sidebar state - this establishes the core shell contract and layout behavior.
2. Mobile right drawer - this extends the same content contract to smaller screens without changing page ownership.
3. Existing shell panel regression - this proves Chat, Observability, preview, no-panel pages, and settings routes behave correctly before migrating more pages.
4. Local inspector migration - this moves existing local contextual right panels to the shell contract in bounded page groups.

The implementation workspace should cover this full feature, with AC slices ordered by shell foundation, mobile drawer, current `rightPanel` regression, then local inspector migration. The migration can split by page group to keep file ownership and verification clear.

== Risks And Open Questions
- The shell currently mixes navigation, workspace selector, utility popovers, agent launcher, and right panel rendering in one large component; edits must stay narrow to avoid unrelated shell regressions.
- Migrating all existing local right inspectors makes this a wider frontend feature; scope control should come from grouped AC slices, not by excluding the migration.
- The project currently has only `en-US` i18n resources, so new labels will follow the existing locale shape and should not claim full `zh-CN` coverage.
- Mobile drawer behavior must not conflict with the existing mobile left navigation overlay.
- No open product questions remain for the agreed baseline.
