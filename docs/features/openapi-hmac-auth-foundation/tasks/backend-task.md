# Backend Task: OpenAPI HMAC Auth Foundation

## Execution Contract

- feature_slug: openapi-hmac-auth-foundation
- status: done
- execution_mode: solo

## TDD Contract

Backend rows prove RED before GREEN on the selected Lite Spec base worktree and keep chat handler business logic unchanged while middleware takes over signed auth and authorization.

## Task Rows

- AC-OAH-02 | phase: tdd | owner: lead | scope: signed-header parsing, canonical request hashing, constant-time HMAC verification, timestamp skew, and bearer removal in `open_api/auth.rs` | command_ref: CMD-BE-TEST | status: done.
- AC-OAH-03 | phase: tdd | owner: lead | scope: acting-user resolution by `X-Tron-User-Code`, explicit user grants, and capability-operation grants | command_ref: CMD-BE-TEST | status: done.
- AC-OAH-04 | phase: tdd | owner: lead | scope: route metadata registry, uniform denial mapping, and request auth context handoff to existing public handlers | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/openapi-open-app-management/base`.
- Branch: `feature/clouds3n/openapi-open-app-management/base`.

## Integration Notes

- Public route mapping stays limited to current agent-chat endpoints.
- Existing public DTOs and handler payloads should remain stable outside auth changes.

## Open Decisions

- None.
