# Test Plan: Integrated Frontend Runtime

## Command Index

| command_ref | command | purpose | cost | rerun policy |
| --- | --- | --- | --- | --- |
| CMD-HELP | `make help` | Verify root workflow command visibility. | low | Rerun after Makefile changes. |
| CMD-BE-TEST | `make be-test` | Run backend tests, including router and Makefile contract tests. | medium | Rerun after backend or Makefile changes. |
| CMD-FE-CHECK | `make fe-check` | Verify standalone frontend typecheck/build still works. | medium | Rerun after Vite config changes. |
| CMD-APP-BUILD | `make app-build` | Verify integrated frontend build writes backend resources and backend builds. | medium/high | Rerun after build workflow changes. |
| CMD-APP-CLEAN | `make app-clean` | Verify generated frontend build artifacts can be removed through Makefile. | low | Rerun after clean target changes. |
| CMD-APP-RUN | `make app-run PORT=18080` | Smoke verify one-port runtime behavior. | medium/high | Rerun manually after integrated runtime changes; terminate after curl checks. |

## Proof Rows

| AC id | case | red | green | command_ref |
| --- | --- | --- | --- | --- |
| AC-INTEGRATED-RUNTIME-01 | Makefile command visibility | Existing Makefile contract test fails because integrated commands are absent. | `make help` and Makefile contract test list integrated commands. | CMD-HELP, CMD-BE-TEST |
| AC-INTEGRATED-RUNTIME-02 | Integrated output path and API-mode defaults | `make app-build` target and Vite output override are absent. | `make app-build` sets same-origin API base, `TRON_FRONTEND_OUT_DIR`, and integrated API-mode defaults while standalone `make fe-check` still succeeds. | CMD-FE-CHECK, CMD-APP-BUILD |
| AC-INTEGRATED-RUNTIME-03 | SPA and assets served by backend | Router tests for `/`, client route, and asset path fail because backend has no frontend static handling. | Router tests pass using configured frontend resource fixtures. | CMD-BE-TEST |
| AC-INTEGRATED-RUNTIME-04 | API boundary 404 | Unknown `/api` route would fall through to frontend fallback or default empty 404. | Unknown `/api` route returns `{ code, message, data }` JSON envelope. | CMD-BE-TEST |
