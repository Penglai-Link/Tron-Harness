# Acceptance: OpenAPI HMAC Auth Foundation

## Acceptance Index

- AC-OAH-01 [must] [database] Open-app foundation stores one active credential, explicit user grants, capability grants, and replay nonces with rotation-safe history; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAH-02 [must] [security] Formal `/open-api/v1/*` accepts only valid `TRON-HMAC-SHA256-V1` headers and rejects bearer or legacy raw app-key auth; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAH-03 [must] [security] Valid signatures still require active explicit `X-Tron-User-Code` grants and matching capability-operation grants before auth context creation; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAH-04 [must] [api] Current public agent-chat routes resolve stable capability-operation metadata and return uniform auth or authorization envelopes before handler business logic; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAH-05 [must] [regression] Public open-api tests and CORS handling recognize the signed-header contract without breaking current route wiring; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
