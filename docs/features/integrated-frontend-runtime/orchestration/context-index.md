# Context Index: Integrated Frontend Runtime

## Primary Files

- `spec/feature-spec.md`: integrated runtime scope and routing model.
- `spec/acceptance.md`: AC rows.
- `spec/test-plan.md`: command aliases and proof rows.
- `tasks/integrated-runtime-task.md`: active execution slice.
- `reports/integrated-runtime-report.md`: RED/GREEN/VERIFY evidence.

## Repository Touchpoints

- `Makefile`: integrated app workflow.
- `tron-frontend/vite.config.ts`: optional frontend output directory override.
- `tron-backend/src/config.rs`: frontend resource directory configuration.
- `tron-backend/src/config/env_config.rs`: environment-variable configuration source and runtime overrides.
- `tron-backend/src/config/file.rs`: YAML configuration file parsing and file override application.
- `tron-backend/src/adapters/inbound/http/frontend.rs`: SPA/static/API boundary routing.
- `tron-backend/src/app.rs`: router integration.
- `tron-backend/tests/frontend_static.rs`: route boundary tests.
- `tron-backend/tests/makefile_contract.rs`: Makefile command visibility.
