# Test Scope: Shared UI Feedback

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Scope Decision

- status: approved_with_defaults
- profile: standard
- rationale: The change is frontend shared UI infrastructure with user-visible interaction behavior. It needs type/build regression plus one browser runtime path.

## Recommended Scope

Standard frontend scope:

- TypeScript and production build through the Makefile.
- Playwright browser runtime test for shared feedback behavior.
- No backend, database, container, or external service tests.

## Included Test Layers

- Frontend compile/typecheck/build.
- Browser runtime interaction test.
- Source-level API/export assertion through TypeScript compile and Playwright import path usage.

## Excluded Test Layers

- Backend tests: not applicable because no backend behavior changes.
- Database tests: not applicable because no persisted state changes.
- Performance tests: not applicable for a tiny transient UI viewport.
- Full app-wide page refactor: excluded because this feature creates the shared primitive first; individual feature migrations follow later.

## Environment Requirements

- Node dependencies already installed under `tron-frontend`.
- Playwright browser runtime available through `npm --prefix tron-frontend exec playwright`.
- Vite dev server can run on an ephemeral localhost port during the browser test.

## Test Boundary

| boundary | in scope | out of scope |
| --- | --- | --- |
| Shared UI API | Provider, hook, exports, tones, dismiss API | Global migration of all existing `setNotice` usage |
| Runtime UI | Top-right floating viewport, visible text, close control, auto-dismiss | Visual screenshot approval |
| App integration | Root provider mount and one shared-shell action | User-management Export/Save loading work |

## Regression Command Candidates

- `rtk make fe-check`
- `rtk npm --prefix tron-frontend exec playwright -- test shared-ui-feedback.spec.ts --reporter=line`

## Evidence Required For Completion

- RED failure evidence for missing shared feedback behavior.
- GREEN pass evidence for frontend typecheck/build.
- Browser evidence with concrete clicks, no input requirement, no network requirement, visible feedback, placement, and auto-dismiss assertions.

## Open Decisions

- None.
