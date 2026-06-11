# Feature Spec: Role Matrix Management

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

This feature implements `/admin/roles` as a real operator workspace instead of a placeholder permission table. The delivered direction is full-stack within the existing Tron admin boundary: keep role-matrix interactions rich on the page, add repository-owned role catalog APIs plus SQLite persistence, and preserve compatibility with the fixed role-code contract already used by `user-management-model`.

## Feature Identity And Context

- feature_slug: role-matrix-mgmt
- feature_name: Role Matrix Management
- status: approved_with_defaults
- owner: clouds3n
- source_request: use `eliteforge-codex-superpower` to create feature `role-matrix-mgmt` and implement `tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx`, making all page interactions real and keeping it related to user-management
- main_agent: Codex lead agent
- dependency_features: `user-management-model`
- dependency_contracts: fixed role codes `ADMIN`, `WORKSPACE_ADMIN`, `DEVELOPER`, `VIEWER`; existing admin shell routes `/admin/users`, `/admin/roles`, `/admin/audit-logs`; shared `useToast`; admin read adapters `listRoles` and `listPermissionDomains`; uniform backend response envelope

## Requirement

- user_story: As an admin operator, I need `/admin/roles` to behave like a usable role-management workspace rather than a static matrix demo.
- intent: Replace placeholder buttons and local-only notices with meaningful page interactions for searching, filtering, inline renaming of custom roles, toggling permissions, creating, cloning, saving, discarding, and deleting role drafts inside one main workspace, then persist the saved catalog through backend APIs.
- outcome: Operators can manage a permission matrix workflow on the page, keep saved role catalogs across reloads through backend persistence, and review role-management data without redundant sub-page tags, summary cards, a dedicated right-side inspector, or an inline metadata editor.

## Product Source Slices

- PRD feature inventory slice: Administration and Workspace Governance.
- Product page slices: `/admin/roles`, `/admin/users`.
- Product rule slices: fixed role-code compatibility with `user-management-model`, full-catalog replacement save, and route-local admin shell continuity.
- Product exception slices: deny silent removal of system roles and keep non-persisted draft actions visible through concrete page feedback.

## Scope Boundary

### Assumptions

- `user-management-model` remains the authority for user assignment flows and fixed role-code consumption.
- Role edits in this feature replace the entire role catalog snapshot and do not yet introduce partial role mutation endpoints or per-field audit history.

### In Scope

- Rebuild `RoleMatrixPage.tsx` into a complete interactive page.
- Add functional search, role-type filtering, and domain focus.
- Add working flows for role selection, inline custom-role naming, permission toggling, add custom role, clone role, reset role, delete custom role, save draft, and discard changes.
- Introduce backend endpoints for permission-domain reads, role-catalog reads, and whole-catalog replacement writes.
- Persist saved role drafts in SQLite and restore them after reload.
- Surface relationship context to user-management through role codes and current user assignment counts.
- Add focused backend and browser verification for the catalog contract, persistence, and visible page interactions.

### Out Of Scope

- Changing `user-management-model` accepted role codes or user assignment API contracts.
- API authorization or RBAC enforcement changes.
- Audit-log persistence for role edits beyond existing frontend navigation.

### Constraints

- Preserve the current admin shell, route registration, and shared toast contract.
- Preserve the existing uniform API envelope and fixed role-code compatibility.
- Do not introduce placeholder buttons, no-op filters, or screen-reader-only feedback as the primary interaction result.

## Feature Model

The page manages a role catalog composed of system roles and persisted custom roles. Each role is identified by a stable role code and carries saved metadata plus a permission matrix keyed by permission domain and permission code. The current UI iteration edits catalog composition and permission grants from the matrix itself; inline metadata editing is intentionally deferred. System-role identity must stay stable so `user-management-model` can continue to reference the fixed role codes safely.

The operator workflow has two layers:

1. Read model: role catalog and permission-domain list from the current admin read adapters, with user-assignment counts derived from the current user dataset.
2. Draft model: a page-local working copy that supports mutation, dirty tracking, save baseline updates, discard, and reload recovery.
3. Persistence model: the backend role catalog stored in SQLite and replaced through `PUT /api/v1/admin/roles`.

The page keeps three invariants:

- Stable role codes that already participate in user-management cannot be silently changed or removed.
- Every visible action must produce a concrete result: state change, toast feedback, or persisted draft update.
- Selected custom roles can be renamed inline inside the matrix row, and permission toggles operate per role and domain directly inside the matrix without secondary editor surfaces.

## Page And Module Surface

- UI: `tron-frontend/src/domain/admin/pages/RoleMatrixPage.tsx`
- Admin API: `tron-frontend/src/domain/admin/api/adminApi.ts`
- Shared UI: `tron-frontend/src/shared/ui/Badge.tsx`, `tron-frontend/src/shared/ui/Button.tsx`, `tron-frontend/src/shared/ui/Forms.tsx`, `tron-frontend/src/shared/ui/Tabs.tsx`, `tron-frontend/src/shared/ui/Toast.tsx`
- Shared layout: `tron-frontend/src/shared/layout/PageHeader.tsx`, `tron-frontend/src/shared/layout/Shell.tsx`, `tron-frontend/src/shared/layout/SurfaceCard.tsx`
- Admin reads: `tron-frontend/src/domain/admin/hooks/useRoleList.ts`, `tron-frontend/src/domain/admin/hooks/usePermissionDomainList.ts`, `tron-frontend/src/domain/admin/hooks/useUserList.ts`
- Admin constants and models: `tron-frontend/src/domain/admin/constants/adminPageOptions.ts`, `tron-frontend/src/domain/admin/constants/permissionDomains.ts`, `tron-frontend/src/domain/admin/model/AdminModel.ts`
- Admin mock data: `tron-frontend/src/domain/admin/mock/data.ts`
- Backend application: `tron-backend/src/application/role.rs`, `tron-backend/src/domain/role.rs`
- Backend transport: `tron-backend/src/adapters/inbound/http/admin/roles/**`
- Backend persistence: `tron-backend/src/adapters/outbound/persistence/sqlite/role_repository.rs`, `tron-backend/migrations/20260515_0004_create_role_management_roles.sql`, `tron-backend/migrations/20260515_0005_create_role_management_permissions.sql`
- Route continuity: `tron-frontend/src/router/routes.tsx`, `tron-frontend/src/router/pageRegistry.ts`
- Verification: `tron-backend/tests/role_matrix_api.rs`, `tron-frontend/tests/role-matrix-smoke.spec.ts`

UI surface in scope:

- Search, type filter, domain filter, changed-only toggle
- Permission matrix with role rows and cell-level permission toggles
- Inline role-name input for the selected custom row so clone/add flows can be renamed without reopening a detail panel
- Top actions for `Add Role`, `Clone`, `Reset`, `Discard`, `Save`, and a red double-confirm `Delete`
- Debounced search input and debounced high-frequency action buttons

## Repository Findings

- `RoleMatrixPage.tsx` originally kept only local `overrides` state, a placeholder filter button, and `sr-only` notice text; it did not close the interaction loop for visible controls.
- `useRoleList` and `usePermissionDomainList` are read adapters and fall back to mock data when admin API mode is unavailable.
- `user-management-model` explicitly treats role definitions and permissions as a separate feature while still depending on fixed role codes for assignments.
- The shared toast surface already exists and is the repo-standard feedback mechanism for admin interactions.
- `routes.tsx` and `pageRegistry.ts` already register `/admin/roles`, so the feature can stay route-local.
- The existing frontend test suite already boots Vite from Playwright specs, which is the fastest path for concrete interaction evidence here.
- The backend already follows a SQLite repository plus Axum handler pattern for `user-management-model`, so role-matrix APIs should follow the same shape instead of inventing a parallel contract.

## Open Decisions

- Whole-catalog `PUT` writes instead of per-role patch endpoints; signal: the page already edits a draft snapshot and save applies the whole matrix in one action; modules: `RoleMatrixPage.tsx`, `adminApi.ts`, `role_repository.rs`; target: feature-owned contract; recommendation: replace the catalog snapshot atomically now and defer granular patching; decision: approved.
- User-assignment counts on the role page; signal: the user requested relation to `user-management`; modules: `RoleMatrixPage.tsx`, `useUserList`; target: feature-local derived read model; recommendation: keep feature-local; decision: approved.
- Header-level cross-page selector on `/admin/roles`; signal: review feedback requested removing the `Users / Role Matrix / Audit Logs` tablist from this page; modules: `RoleMatrixPage.tsx`; target: feature-local UI surface; recommendation: keep `/admin/roles` focused on the role matrix itself and rely on the shell navigation for cross-page movement; decision: approved.
- Header status badge and right-side inspector on `/admin/roles`; signal: review feedback requested removing the `Role Matrix` sub-page tag and collapsing the right-side editor into the main content flow; modules: `RoleMatrixPage.tsx`; target: feature-local UI surface; recommendation: keep the header clean and place editing plus action controls inside the main workspace; decision: approved.
- Summary cards, inline metadata editor, and permission-count labels on `/admin/roles`; signal: review feedback requested deleting the four summary cards, deferring the detail editor, and removing `n/4 permissions` labels while adding debounce to search and high-frequency buttons; modules: `RoleMatrixPage.tsx`; target: feature-local UI surface; recommendation: keep the page as a compact action row plus matrix workspace for now; decision: approved.
