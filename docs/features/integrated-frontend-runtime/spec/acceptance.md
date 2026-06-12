# Acceptance: Integrated Frontend Runtime

## Acceptance Criteria Matrix

| id | criterion | evidence type | expected evidence | priority |
| --- | --- | --- | --- | --- |
| AC-INTEGRATED-RUNTIME-01 | The root Makefile exposes integrated build, run, and clean commands while preserving existing frontend and backend commands. | automated | `make help` includes `app-build`, `app-run`, `app-clean`, and existing command entries. | must |
| AC-INTEGRATED-RUNTIME-02 | The frontend build can write production assets directly into the backend resource directory while defaulting integrated targets to same-origin API mode and leaving standalone frontend workflows unchanged. | automated | Vite config test/build evidence shows `TRON_FRONTEND_OUT_DIR` controls output and `make app-build` injects same-origin API mode defaults for integrated builds. | must |
| AC-INTEGRATED-RUNTIME-03 | The backend serves frontend entry and asset files from one backend port for non-API paths. | automated | Router tests prove `/`, client routes, and assets are served from configured frontend resources. | must |
| AC-INTEGRATED-RUNTIME-04 | Unknown `/api` routes return a uniform JSON 404 response instead of frontend HTML. | automated | Router test asserts JSON envelope for `/api/v1/not-exists`. | must |

## Evidence Requirements

- AC-INTEGRATED-RUNTIME-01: `make help` and backend Makefile contract test.
- AC-INTEGRATED-RUNTIME-02: frontend build target in `Makefile` plus `make app-build` or focused build verification, including the integrated API-mode default contract.
- AC-INTEGRATED-RUNTIME-03: backend router static frontend tests.
- AC-INTEGRATED-RUNTIME-04: backend router API 404 boundary test.
