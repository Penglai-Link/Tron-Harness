# Feature Spec: Init Rust Backend

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

Lead agent interpretation: initialize the existing `tron-backend` folder as a Rust backend service for the Tron monorepo. The first backend slice should be small but real: a Cargo package, HTTP runtime, uniform response envelope, configuration loading, a health endpoint, and Makefile backend commands. The user explicitly corrected this workflow to multi-agent mode, so implementation tasks are assigned to worker worktrees and integrated through the Lite Spec `/base` branch.

## Feature Identity

- feature_slug: init-rust-backend
- feature_name: Init Rust Backend
- status: approved_with_defaults
- owner: clouds3n
- source_request: 初始化rust后端项目
- main_agent: lead agent on `feature/clouds3n/init-rust-backend/base`

## Requirement Description

Initialize the backend side of the Rust + React monorepo so backend development can start from a runnable, testable Rust service. The backend should expose a minimal API with the project's uniform response body format and remain aligned with the documented deployment model where the backend eventually serves APIs and frontend static assets from one runtime.

## Goals And Non-Goals

Goals:

- Create a Rust Cargo application under `tron-backend`.
- Provide a lightweight HTTP service with `GET /api/v1/health`.
- Use a uniform response shape: `code`, `message`, and `data`.
- Add configuration defaults and file/environment override support suitable for local development.
- Add backend Makefile targets because project instructions say to use Makefile first.
- Add automated Rust tests that verify the initialization contract.

Non-goals:

- Do not implement business domains such as auth, dashboard, task, model, memory, or automation.
- Do not add database schema or migrations in this initialization slice.
- Do not wire frontend API mode to a live backend route beyond preserving the documented boundary.
- Do not implement static frontend asset serving yet; prepare structure only.

Assumptions:

- Rust 2024 edition is acceptable because the installed toolchain supports it.
- SQLite remains the default future persistence option, but no database access is needed for health checks.
- Axum and Tokio are acceptable foundation choices for a small HTTP backend.

## User Roles And Permissions

This initialization introduces no user-facing roles, login sessions, tenancy rules, or permission decisions. `GET /api/v1/health` is public operational metadata and returns no sensitive data.

## Page Descriptions

No frontend pages are introduced or modified. The backend entry point is an HTTP endpoint:

- `GET /api/v1/health`: returns service status, service name, and version using the uniform response envelope.

## Domain Model

Bounded context: backend platform bootstrap.

- Aggregate root: not applicable.
- Entity: not applicable.
- Value objects:
  - `ApiResponse<T>`: uniform response envelope.
  - `HealthData`: service health payload.
  - `AppConfig`: service runtime configuration.
- Domain service:
  - HTTP router composition.
  - Configuration loading.
- Invariants:
  - Successful API responses use `code = "SUCCESS"`.
  - API payloads are nested under `data`.
  - Health output must be deterministic enough for automated tests.

## Module And Boundary Map

Backend modules:

- `tron-backend/src/main.rs`: binary entry point.
- `tron-backend/src/lib.rs`: application module exports for tests and future reuse.
- `tron-backend/src/api`: response envelope and route handlers.
- `tron-backend/src/config`: runtime configuration loading and defaults.

Repository boundaries:

- Root `Makefile` owns backend commands.
- `tron-frontend` remains unchanged in this feature.
- Future generated frontend clients must treat backend DTOs as stable API contracts, not import backend Rust types directly.

Common capability extraction:

- `ApiResponse<T>` should live in backend shared API code immediately because every future endpoint must use it.

## Repository Findings

- `AGENTS.md` requires using `Makefile` first and documents the uniform response body format.
- `AGENTS.md` declares Git Spec follows `$eliteforge-git-feature-oriented-lite-spec`.
- Root `Makefile` currently exposes only frontend commands.
- `docs/tech-stack.md` describes a Rust + React monorepo where the backend eventually serves API endpoints and frontend static assets.
- `tron-backend/docs/tech-stack.md` declares Rust, Cargo, SQLite, and YAML config.
- `tron-backend/docs/spec/rust-coding-spec.md` requires modular architecture, pluggable storage, SQLite default, and one runtime service.
- `tron-backend` had docs and `.gitignore` but no Cargo package before this feature.
- No existing `docs/features/*/spec/feature-spec.md` files were present during discovery.

## Feature Dependencies

Strong feature dependencies: none found after scanning 0 feature-spec.md files under `docs/features`. This slice initializes backend platform structure and does not depend on another feature's domain model, API/schema contract, workflow state, permission model, migration, generated client, shared component, or acceptance behavior.

## Open Decisions

| decision id | candidate common capability | reuse signal | affected modules | proposed shared module | recommendation | user decision |
| --- | --- | --- | --- | --- | --- | --- |
| OD-COMMON-01 | uniform API response envelope | every future backend endpoint must follow `AGENTS.md` response format | backend API handlers and future frontend API clients | `tron-backend/src/api/response.rs` | extract now | approved_with_defaults |
