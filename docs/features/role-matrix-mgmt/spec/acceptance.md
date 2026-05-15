# Acceptance: Role Matrix Management

## Acceptance Index

- AC-RM-01 [must] [business] Role Matrix shows system and custom roles with permission coverage, assignment counts, and visible draft state directly in the matrix without separate summary cards; proof: pending.
- AC-RM-02 [must] [interaction] Search, role-type filter, domain filter, and changed-only toggle all update visible page state meaningfully, and the redundant cross-page tab selector is absent from `/admin/roles`; proof: pending.
- AC-RM-03 [must] [interaction] Operators can select roles, rename selected custom roles inline, and toggle permissions directly from the matrix without placeholder actions or a separate editor surface; proof: pending.
- AC-RM-04 [must] [interaction] Add custom role, clone, inline-rename cloned roles, reset, and delete flows complete with valid state changes and shared toast feedback; proof: pending.
- AC-RM-05 [must] [interaction] Save Draft writes the role catalog through the backend API, reload restores the persisted catalog, and Discard Changes restores the last saved baseline; proof: pending.
- AC-RM-06 [must] [business] Role-matrix APIs expose permission domains, a seeded role catalog, and persisted replacement writes behind the uniform admin envelope; proof: pending.
- AC-RM-07 [must] [regression] `/admin/roles` remains route-safe, mock/API compatible, and aligned with user-management fixed role codes; proof: pending.

## Open Decisions

- None.
