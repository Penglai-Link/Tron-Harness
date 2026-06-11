# Test Report: OpenAPI HMAC Auth Foundation

## Summary

Backend regression now proves the signed-header contract across current public agent-chat routes, including CORS preflight, bearer rejection, nonce replay protection, explicit grants, and stream or replay flows.

## Evidence Rows

- AC-OAH-05 | red: public open-api regression fixtures and preflight expectations still depended on legacy headers before the HMAC foundation landed. | green: `rtk make be-test` and `rtk cargo test --manifest-path tron-backend/Cargo.toml --test open_api_chat_api -- --nocapture` passed, proving CORS preflight allows the signed headers and the open-api integration helpers exercise current session, message, stream, and AGUI replay routes under the HMAC-only contract. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/tests/open_api_chat_api.rs`
- `tron-backend/tests/common/mod.rs`
- `tron-backend/src/adapters/inbound/open_api/auth.rs`

## Commands

- `rtk make be-test`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test open_api_chat_api -- --nocapture`

## Blockers

None.
