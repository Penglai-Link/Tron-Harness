# Acceptance: Model Management

## Acceptance Index

- AC-MODEL-01 [must] [business] Workspace catalogs resolve visible rows into current-workspace `Local`, `Inherited`, or `Override` states and show `Inherited From` as `Platform` or the full source workspace lineage path; proof: backend API tests and browser smoke passed on 2026-05-24.
- AC-MODEL-02 [must] [interaction] Lifecycle actions and platform or workspace create/detail verify all work without implicit save; proof: backend API tests and browser smoke passed on 2026-05-25.
- AC-MODEL-03 [must] [business] Defaults resolve per coarse `modelType`, and the first enabled platform model in an empty slot becomes the explicit platform default; proof: backend API tests and browser smoke passed on 2026-05-25.
- AC-MODEL-04 [must] [api] Backend, frontend, mocks, and persistence now follow the latest contract only and do not preserve the old restore or shadow compatibility model; proof: compile, API tests, browser smoke, and feature-doc validation passed on 2026-05-24.

## Open Decisions

- None.
