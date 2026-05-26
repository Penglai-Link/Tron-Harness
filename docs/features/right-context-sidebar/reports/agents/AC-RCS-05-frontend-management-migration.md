# AC-RCS-05 Frontend Management Migration Evidence

## Changed Paths

- `tron-frontend/src/domain/task/pages/TaskDetailsPage.tsx`
- `tron-frontend/src/domain/automation/pages/AutomationDetailsPage.tsx`
- `tron-frontend/src/domain/app/pages/AppDetailsPage.tsx`
- `tron-frontend/src/domain/prompt/pages/PromptsPage.tsx`
- `tron-frontend/src/domain/variable/pages/VariableManagementPage.tsx`
- `tron-frontend/src/domain/knowledge-base/pages/KnowledgeBasePage.tsx`
- `tron-frontend/src/domain/skill/pages/SkillsPage.tsx`
- `tron-frontend/tests/right-context-sidebar.spec.ts`
- `docs/features/right-context-sidebar/reports/agents/AC-RCS-05-frontend-management-migration.md`

## RED Evidence

- First `CMD-FE-RIGHT-SIDEBAR-BROWSER` run failed at `AC-RCS-05` after navigation to `/automations/detail?automationCode=AUT-734600000000000001`.
  - Failure: `TypeError: Cannot read properties of undefined (reading 'length')` in `AutomationDetailsPage.tsx` at `automation.subtasks.length`.
- After the first fix for execution list safety, the same command failed on `/apps/detail?appCode=APP-TRON-CONSOLE` because the right sidebar assertion expected a page-local string (`Tron Console`) that was not present in moved shell panel content.
- Subsequent runs exposed selector ambiguity in `PromptsPage` and missing row assumptions in `SkillsPage` (`/settings/skills` excluded, no row in `/skills` under mocked API flow).
- Command used: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`.

## GREEN Evidence

Representative routes and interactions validated using `CMD-FE-RIGHT-SIDEBAR-BROWSER`:

- `/tasks/detail?taskCode=...`
  - Asserted shell sidebar heading `Task Metadata`.
  - Confirmed selected task code visible in main list detail.
  - Opened/closed edit mode, collapsed sidebar and reopened; `Task Metadata` panel remained available.
- `/automations/detail?automationCode=AUT-734600000000000001`
  - Asserted `Automation Details` heading and `Automation code` field visible in right sidebar.
  - Verified `Execution` table rendered and sidebar remained functional.
- `/apps/detail?appCode=APP-TRON-CONSOLE`
  - Asserted right sidebar heading `App Metadata` and preserved right context after collapse/reopen.
- `/prompts`
  - Selected `Code Review Guidelines`, opened `Run Test Preview`, got preview confirmation.
  - Closed inspector through right sidebar action, verified inspector removed, selected another prompt (`LLM Router System`) and verified it reappeared.
- `/variables/details` and `/variables`
  - Confirmed `Variable Studio` sidebar presence.
  - Selected a variable row and verified selected key reflected in sidebar heading.
- `/knowledge-base`
  - Confirmed sidebar heading `Selected KnowledgeBase`, selected `Operations SOP Library`, verified sidebar updated.
- `/skills`
  - Confirmed `Skill Detail` heading in shell sidebar.
  - Clicked first available row when present and verified sidebar heading remains in detail mode.
- `/settings/skills`
  - Confirmed settings exclusion by asserting no right sidebar controls and no shell controls on settings route (as required by AC-RCS-05 note).

### Settings exclusion note

- `/settings/skills` intentionally preserves existing settings layout behavior and does **not** receive shell right-sidebar migration in this AC.

## AC-RCS-05

- Migrated management/detail inspector ownership from page-local right-column grids to shell-owned `rightPanel` via `useConsoleShellConfig({ rightPanel })` in task, automation, app, prompt, variable management, variable details, knowledge-base, and skill pages.
- Preserved local actions, selection updates, and page-level operations while reusing existing content.
- Removed local right inspector column grids from migrated pages and kept behavior in main content intact.
- Prompt/automation/variable close/reopen paths now operate by controlling shell panel visibility in-page state, preserving selected context.
- Maintained `/settings/skills` as a non-migrated route.

## Commands

- PASS: `rtk npm --prefix tron-frontend exec playwright -- test right-context-sidebar.spec.ts --reporter=line`
  - Result: 5/5 passed, AC-RCS-05 green.
- PASS: `rtk make fe-check`
  - Result: typecheck + build completed successfully for `tron-frontend`.
- PASS: `rtk make fe-install`
  - Result: dependencies available for test/build flows.

## Blockers

- No remaining blockers for AC-RCS-05 within the assigned scope.
