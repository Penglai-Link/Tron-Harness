# API Contract: {{FEATURE_NAME}}

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: pending
- best_practice_defaults_applied: pending
- user_confirmation_status: waiting_human_decision

Summarize the lead agent's first-pass interpretation, recommended defaults, assumptions, and blocking confirmation questions. This section must exist before task planning.

## API Overview

Summarize external and internal API behavior.

## Endpoints

| method | path | purpose | auth | acceptance ids |
| --- | --- | --- | --- | --- |

## Request And Response DTOs

Document each API interface under a separate `###` header. For every interface, specify how request parameters are passed, such as path variables, query parameters, request headers, request body, multipart form data, or mixed methods.

### {{METHOD}} {{PATH}} - {{INTERFACE_NAME}}

- Parameter passing method: {{path variables | query parameters | request headers | request body | multipart form data | mixed}}
- Path variables: {{name, type, required, validation, description}}
- Query parameters: {{name, type, required, validation, description}}
- Request headers: {{name, type, required, validation, description}}
- Request body DTO: {{name, fields, nested types, validation rules}}
- Response body DTO: {{name, fields, nested types, serialization rules}}
- Generated client impact: {{client method name, request type, response type, compatibility notes}}
- Backward compatibility: {{breaking changes, migration path, deprecated fields}}

## Auth And Permissions

Document allowed roles, denied roles, ownership checks, tenancy rules, and audit requirements.

## Error Contract

| condition | status/code | response shape | acceptance ids |
| --- | --- | --- | --- |

## Frontend Backend Boundary

Define frontend responsibilities, backend responsibilities, shared types, route dependencies, and generated client expectations.

## Database And Transaction Boundary

Document transaction boundaries, persistence responsibilities, locking behavior, consistency expectations, and database error mapping.

## Mock Or Adapter Expectations

Document mocks, external adapters, service virtualization, test doubles, and deterministic test seams.

## Open Decisions

- None.
