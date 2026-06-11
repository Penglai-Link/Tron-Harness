# Backend Report: OpenAPI HMAC Auth Foundation

## Summary

The public `/open-api/v1/*` middleware now accepts only signed HMAC requests, resolves acting users by `X-Tron-User-Code`, authorizes by capability-operation grants, and rejects failures through the uniform Tron envelope before handler business logic runs.

## Evidence Rows

- AC-OAH-02 | red: legacy bearer or raw app-key requests could still pass public open-api auth, and HMAC headers were not enforced. | green: `rtk make be-test` passed signed-request coverage proving only valid `TRON-HMAC-SHA256-V1` headers succeed while bearer-only, malformed, stale, replayed, and bad-signature requests fail. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-03 | red: valid signatures could still impersonate any active user or call any route without explicit grants. | green: `rtk make be-test` passed acting-user and capability authorization coverage proving signed requests fail for ungranted users and missing capability-operation grants and succeed only for explicit granted pairs. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-04 | red: public routes had no stable capability-operation mapping and auth failures could bypass the standard Tron response envelope. | green: `rtk make be-test` passed route-mapping and denial-envelope coverage proving current chat routes resolve stable `agent-chat` operations and unauthorized or forbidden failures return `{ code, message, data }` before handler logic executes. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/adapters/inbound/open_api/auth.rs`
- `tron-backend/src/application/open_app/`
- `tron-backend/src/application/open_app.rs`
- `tron-backend/src/domain/open_app/`
- `tron-backend/src/adapters/inbound/http/admin/open_apps/`
- `tron-backend/src/application/user/service/auth_use_cases.rs`
- `tron-backend/src/app.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test open_api_chat_api -- --nocapture`

## Blockers

None.
