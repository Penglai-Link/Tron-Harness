# Test Plan: Role Matrix Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-15

Valid statuses: `waiting_human_decision`, `approved`, `approved_with_defaults`.

## Proof Rows

- AC-RM-01 | layer: browser | case: initial matrix shows normalized roles, assignment counts, permission legend, and draft indicators without separate summary cards | red: `/admin/roles` still behaves like a placeholder table, or redundant summary cards remain above the matrix | green: same browser proof shows populated role rows, legend, and draft indicators while the summary cards stay absent | interaction: navigate to `/admin/roles` -> inspect role rows and legend -> confirm summary-card labels do not render -> observe role, permission-domain, and related-user reads | command_ref: CMD-RM-BROWSER.
- AC-RM-02 | layer: browser | case: debounced search, type filter, domain filter, and changed-only toggle all change the visible result set, and the redundant `Users / Role Matrix / Audit Logs` tablist is absent | red: visible controls remain placeholders, or the removed cross-page selector still renders on the page | green: same browser proof shows filtered matrix state changes after debounced search settles and confirms the deleted tab selector does not render | interaction: load `/admin/roles` -> verify `Users` and `Audit Logs` tabs are absent -> type into search -> switch role and domain filters -> toggle changed-only view | command_ref: CMD-RM-BROWSER.
- AC-RM-03 | layer: browser | case: role selection, inline custom-role renaming, and permission toggles update working draft state directly from the matrix after the right-side inspector and detail editor are removed | red: matrix cells cannot drive a real draft update, selected custom roles cannot be renamed inline, the old inspector or detail editor still renders, or permission-count labels like `4/4 permissions` remain | green: same browser proof shows selection, inline custom-role name edits, permission changes, no `INSPECTOR` sidebar, no detail editor card, and no permission-count labels | interaction: select or create a custom role -> edit the visible row name input -> toggle matrix permission chips -> observe dirty state -> confirm removed editor surfaces stay absent | command_ref: CMD-RM-BROWSER.
- AC-RM-04 | layer: browser | case: add custom role, clone selected role, inline-rename the cloned role, reset a changed role, and delete a custom role all work without fake buttons, with equal-width actions grouped beside save controls, delete requiring a second confirmation, and repeated rapid clicks suppressed by debounce | red: visible role action buttons still emit only placeholder notices, cloned rows cannot be renamed, create duplicate rapid-fire state changes, or delete fires without a second confirm step | green: same browser proof shows debounced add/toggle behavior, new rows, cloned rows with editable names, reset recovery, red delete styling, confirm-before-delete behavior, and shared toast feedback from the top action row | interaction: rapidly click Add Role -> toggle matrix permissions -> use Clone -> rename the visible cloned row input -> use Reset / Delete -> verify Delete changes to Confirm before removal -> verify state changes | command_ref: CMD-RM-BROWSER.
- AC-RM-05 | layer: browser | case: save writes the catalog through `PUT /api/v1/admin/roles`, reload restores the persisted catalog, and discard reverts unsaved changes | red: save does not issue a role-catalog write, reloading loses the saved state, or discard does not restore the saved baseline | green: same browser proof shows backend write traffic, saved state surviving reload, and discard recovering the last baseline | interaction: change a role -> click Save -> verify `PUT /api/v1/admin/roles` -> reload page -> mutate again -> click Discard -> verify restored values | command_ref: CMD-RM-BROWSER.
- AC-RM-06 | layer: backend | case: role-matrix APIs expose permission domains, seeded roles, and persisted replacement writes | red: permission-domain or role-catalog routes return 404 or replacement writes do not survive a fresh router on the same SQLite database | green: backend test suite proves `GET /api/v1/admin/permission-domains`, `GET /api/v1/admin/roles`, and `PUT /api/v1/admin/roles` with persistence across reload | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-RM-07 | layer: browser | case: role matrix keeps the registered admin route and fixed role-code compatibility after the header sub-page tag and right-side inspector are removed | red: deleting the header badge or sidebar breaks `/admin/roles` rendering, saved role state, or fixed role-code visibility | green: browser proof keeps `/admin/roles` usable in API mode after the layout cleanup while `make fe-check` also passes | interaction: load `/admin/roles` -> confirm the `Role Matrix` badge and `INSPECTOR` sidebar are absent -> confirm headings, role rows, and saved role state still render | command_ref: CMD-RM-BROWSER.

## Command Index

- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: red/green/verify/final | rerun: affected frontend changes | expect: frontend typecheck and build pass.
- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: green/verify/final | rerun: affected backend changes | expect: backend routes, repository persistence, and role-matrix API tests pass.
- CMD-RM-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test role-matrix-smoke --reporter=line | cost: slow/browser | when: verify | rerun: review-if-stale | expect: clicks, inputs, route assertions, backend role-catalog writes, visible feedback, and role-matrix interaction evidence.

## Evidence Rules

- Every `AC-RM-*` row must have final evidence in `reports/frontend-result.md`.
- Browser evidence must record concrete clicks, inputs, route or storage assertions, visible feedback, and the test artifact path.
- Backend evidence must record concrete route coverage and persistence proof against SQLite.
- Regression evidence must include `make fe-check`.
