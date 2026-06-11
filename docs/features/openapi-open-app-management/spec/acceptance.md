# Acceptance: OpenAPI Open-App Management

## Acceptance Index

- AC-OAM-01 [must] [api] Platform admin APIs list, create, inspect, update status or metadata, and rotate platform open-apps while revealing cleartext `appKey` only in create or rotate responses; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAM-02 [must] [api] Platform admin APIs expose the current business openapi capability catalog grouped for operator use and backed by the same registry used by runtime route authorization; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAM-03 [must] [security] Platform admin APIs replace explicit acting-user grants and capability-operation grants for one open-app, rejecting invalid or inactive user targets while preserving the standard response envelope; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAM-04 [must] [business] Settings openapi page shows current open-apps, grouped capability catalog, explicit user grants, and one-time key reveal UX for create or rotate within the settings shell; proof: @ref:spec/test-plan.md#proof-rows.
- AC-OAM-05 [must] [regression] Automated checks and browser evidence prove the end-to-end platform management flow without reintroducing app-management naming or legacy raw-key management assumptions; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
