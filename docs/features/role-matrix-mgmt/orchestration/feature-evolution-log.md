# Feature Evolution Log: Role Matrix Management

## Summary

- feature_slug: role-matrix-mgmt
- mode: new_feature_zero_one
- note: no evolution entries yet; this workspace started as a new feature

## Change History

- EV-01 | 2026-05-15 | scope: role-matrix persistence boundary | reason: original 0-1 delivery stopped at frontend interactions, but the user clarified that backend interfaces were required too | change: added Axum role-matrix endpoints, SQLite persistence, frontend `PUT /api/v1/admin/roles` integration, and matching backend/browser proof | impact: feature is now full-stack instead of frontend-only.
- EV-02 | 2026-05-15 | scope: page-header interaction surface | reason: review feedback requested deleting the `Users / Role Matrix / Audit Logs` selector from `/admin/roles` | change: removed the header-level tablist from `RoleMatrixPage.tsx`, updated browser proof to assert absence, and synchronized feature docs | impact: page header is simpler and the role matrix now relies on shell navigation instead of an in-page cross-page selector.
- EV-03 | 2026-05-15 | scope: page layout cleanup | reason: review feedback requested removing the `Role Matrix` header tag and the right-side inspector, then moving the role actions beside save controls | change: removed the header badge, collapsed the editor into the main content flow, moved Clone / Reset Role / Delete into the page action row, and updated browser proof plus feature docs | impact: the role matrix now edits roles inside one main workspace column without a dedicated sidebar.
- EV-04 | 2026-05-15 | scope: action-row ergonomics | reason: review feedback requested better action ordering, simpler labels, equal button widths, and safer delete behavior | change: reordered the header buttons, shortened labels to Add Role / Reset / Discard / Save, styled Delete as red, and required a second click on Confirm before removal | impact: the action row is denser, more consistent, and less error-prone for destructive operations.
- EV-05 | 2026-05-15 | scope: matrix-first simplification | reason: review feedback requested deleting the four summary cards, removing the inline detail editor for now, dropping `n/4 permissions` labels, and debouncing search plus high-frequency buttons | change: reduced `/admin/roles` to the action row plus permission matrix, removed summary/detail surfaces and permission-count labels, added debounced search settling plus button/toggle click guards, and refreshed browser proof plus docs | impact: the page is now a denser matrix workspace with lower accidental double-trigger risk and less redundant chrome.
- EV-06 | 2026-05-15 | scope: clone follow-up ergonomics | reason: review feedback requested that cloned roles remain renameable even after the detail editor card was removed | change: added inline role-name editing for the selected custom row, autofocus on add/clone, and browser proof that a cloned role can be renamed before further actions | impact: clone remains useful in the simplified matrix-only layout without restoring the larger editor surface.

## Downstream Artifact Migration

- None.

## Residual Risks

- The write contract currently replaces the full role catalog snapshot in one request; future concurrent editing would need versioning or finer-grained mutation endpoints.
