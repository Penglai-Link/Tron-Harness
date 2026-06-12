# Report: Integrated Frontend Runtime

## Evidence Rows

| AC id | phase | command_ref | result | evidence |
| --- | --- | --- | --- | --- |
| AC-INTEGRATED-RUNTIME-03, AC-INTEGRATED-RUNTIME-04 | RED | CMD-BE-TEST | failed as expected | `make be-test` failed before implementation because `tests/frontend_static.rs` referenced missing `AppConfig.frontend_dist_dir`, proving the backend could not configure frontend resources yet. |
| AC-INTEGRATED-RUNTIME-01, AC-INTEGRATED-RUNTIME-02, AC-INTEGRATED-RUNTIME-03, AC-INTEGRATED-RUNTIME-04 | GREEN | CMD-BE-TEST | passed | `make be-test` passed after adding Makefile contract coverage, configurable frontend resource routing, SPA/static fallback tests, and config module splitting to satisfy the Rust architecture guard. |
| AC-INTEGRATED-RUNTIME-01 | VERIFY | CMD-HELP | passed | `make help` lists `app-build`, `app-run`, and `app-clean` while preserving existing frontend and backend command groups. |
| AC-INTEGRATED-RUNTIME-02 | VERIFY | CMD-FE-CHECK | passed | `make fe-check` passed; standalone frontend typecheck/build still emits to `tron-frontend/dist`. npm printed the existing `always-auth` warning and Vite printed the existing large chunk warning. |
| AC-INTEGRATED-RUNTIME-02 | VERIFY | CMD-APP-BUILD | passed | `make app-build` passed. The command injects `VITE_TRON_API_BASE_URL=`, `VITE_TRON_DATA_SOURCE=api`, `VITE_TRON_AUTH_DATA_SOURCE=api`, `VITE_TRON_ADMIN_DATA_SOURCE=api`, and `TRON_FRONTEND_OUT_DIR=../tron-backend/resources/frontend`, emits frontend assets under `tron-backend/resources/frontend`, and completes `cargo build`. |
| AC-INTEGRATED-RUNTIME-03, AC-INTEGRATED-RUNTIME-04 | VERIFY | CMD-APP-RUN | passed | `make app-run PORT=8080` was started with the integrated API-mode build defaults plus temporary SQLite and graph storage paths to avoid local state collisions. Curl smoke verified `GET /` returns `text/html`, `GET /api/v1/health` returns `SUCCESS` JSON, and `GET /api/v1/not-exists` returns JSON 404 `{ code, message, data }`. The process was intentionally interrupted after verification, so Make stopped with an interrupt exit. |
