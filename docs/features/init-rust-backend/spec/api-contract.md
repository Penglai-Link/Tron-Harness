# API Contract: Init Rust Backend

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

The initial backend exposes one operational endpoint and a reusable response envelope that future APIs must reuse. This keeps the implementation aligned with `AGENTS.md` before business endpoints exist.

## API Overview

The backend exposes `GET /api/v1/health` for local development and service readiness checks. The endpoint is public and returns only non-sensitive service metadata.

## Endpoints

| method | path | purpose | auth | acceptance ids |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/health` | report backend runtime health | none | AC-BACKEND-01, AC-BACKEND-02 |

## Request And Response DTOs

### GET /api/v1/health - Health Check

- Parameter passing method: none
- Path variables: none
- Query parameters: none
- Request headers: none required
- Request body DTO: none
- Response body DTO: `ApiResponse<HealthData>`
  - `code: string`, success value `SUCCESS`
  - `message: string`, success value `ok`
  - `data.status: string`, success value `ok`
  - `data.service: string`, default value `tron-backend`
  - `data.version: string`, package version
- Generated client impact: future client method can be `getHealth`; no generated client is created in this slice.
- Backward compatibility: this is the first backend API contract.

## Auth And Permissions

No authentication or authorization is required for `GET /api/v1/health`. Future business APIs must define permission and tenancy checks separately.

## Error Contract

| condition | status/code | response shape | acceptance ids |
| --- | --- | --- | --- |
| health request succeeds | HTTP 200 / `SUCCESS` | `{ code, message, data }` | AC-BACKEND-01, AC-BACKEND-02 |
| route is unknown | HTTP 404 | Axum default 404 response | AC-BACKEND-03 |

## Frontend Backend Boundary

The frontend remains mock-data-first in this feature. The backend owns runtime API behavior and response shape. The frontend may later consume `/api/v1/health` through the existing `VITE_TRON_API_BASE_URL` mechanism, but no frontend route changes are required now.

## Database And Transaction Boundary

No database transaction boundary is introduced. The health route is stateless.

## Mock Or Adapter Expectations

Tests should call the Axum router directly without opening a real port. No external service adapters or mocks are required.

## Open Decisions

- None.
