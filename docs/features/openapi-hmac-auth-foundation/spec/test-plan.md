# Test Plan: OpenAPI HMAC Auth Foundation

## Test Plan Decision

- status: approved_with_defaults
- approved_by: user approved the product direction and delegated bounded feature implementation details to Codex
- approved_at: 2026-06-11

## Proof Rows

- AC-OAH-01 | layer: database | case: open-app schema and single-active-credential persistence | red: no repository or schema can persist platform open-app identities, active credentials, grants, or replay nonces | green: backend tests prove migrations create the new tables, enforce one active credential, persist grants, and reject duplicate app nonce reuse | command_ref: CMD-BE-TEST.
- AC-OAH-02 | layer: security | case: signed request verification and bearer removal | red: legacy bearer or raw app-key requests can still pass public open-api auth, and HMAC headers are not verified | green: backend API tests prove only valid `TRON-HMAC-SHA256-V1` headers succeed while bearer-only, legacy, malformed, stale, and bad-signature requests fail | command_ref: CMD-BE-TEST.
- AC-OAH-03 | layer: security | case: acting user and capability authorization after signature success | red: valid signatures can impersonate any active user or any route without explicit grants | green: backend tests prove signed requests fail for inactive or ungranted users and for missing capability-operation grants, and pass only for explicit granted pairs | command_ref: CMD-BE-TEST.
- AC-OAH-04 | layer: api | case: route metadata mapping and uniform denial envelope | red: public routes have no stable capability-operation mapping and auth failures bypass the uniform Tron error envelope | green: backend tests prove current chat routes resolve the expected `agent-chat` operation metadata and denied requests return stable `{ code, message, data }` envelopes before handler logic runs | command_ref: CMD-BE-TEST.
- AC-OAH-05 | layer: backend | case: signed-header regression and preflight updates | red: current integration tests and browser-oriented preflight expectations still depend on legacy open-api headers | green: backend tests prove CORS preflight allows the signed headers and the updated open-api test helpers exercise the new contract across current public routes | command_ref: CMD-BE-TEST.

## Command Index

- CMD-BE-TEST | command: `rtk make be-test` | cost: medium | when: red-green-verify | rerun: after backend auth, repository, migration, or public route changes | expect: backend tests pass including open-app schema and open-api signed auth coverage.
- CMD-BE-CHECK | command: `rtk make be-check` | cost: medium | when: verify | rerun: after backend module or route wiring changes | expect: backend compiles cleanly.
- CMD-BE-OPENAPI-FOCUSED | command: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test open_api_chat_api -- --nocapture` | cost: medium | when: focused-verify | rerun: after `CMD-BE-TEST` passes and only open-api auth assertions need a tighter loop | expect: public open-api integration tests pass with the signed header helpers.
- CMD-DOCS-WORKTREE | command: `rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/openapi-hmac-auth-foundation --phase worktrees` | cost: low | when: docs | rerun: after spec, tasks, or worktree-plan changes | expect: feature docs pass the worktree-phase validator.

## Evidence Rules

- Security evidence must distinguish signature failure from authorization failure.
- Replay evidence must show the second request or duplicate nonce reservation is rejected by the same app scope.
- Error-envelope evidence must inspect JSON `code`, `message`, and `data`.
- Makefile-first commands remain the primary proof path; focused cargo reruns are secondary loops only after the baseline passes.

## Open Decisions

- None.
