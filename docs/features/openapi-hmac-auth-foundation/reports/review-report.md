# Review Report: OpenAPI HMAC Auth Foundation

## Summary

Review confirmed the HMAC foundation closes the original public-auth gap without widening business scope: one persisted open-app auth state, HMAC-only middleware, explicit acting-user grants, explicit capability grants, and stable current route mapping.

## Evidence Rows

- AC-OAH-01 | red: review started from a workspace that had no persisted open-app credential, grant, or nonce replay ledger. | green: review confirmed the new repository and migration keep one active credential, retain revoked history, load grants into auth state, and reject duplicate nonce inserts under passing backend evidence. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-02 | red: review found the old public auth path still exposed bearer fallback and lacked full signed-header verification. | green: review confirmed `open_api/auth.rs` now requires `X-Tron-App-Code`, `X-Tron-User-Code`, timestamp, nonce, signature version, and signature headers and maps bearer-only access to `OPEN_API_UNAUTHORIZED`. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-03 | red: review found valid signatures alone were insufficiently bound to explicit user and operation authorization. | green: review confirmed authenticated open-api context is created only after explicit user-grant and capability-grant checks succeed. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-04 | red: review found no stable business grant registry for current public agent-chat routes. | green: review confirmed route metadata resolves current `agent-chat` operations before handlers run and denied requests return the standard Tron envelope. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-OAH-05 | red: review required explicit regression proof that public route helpers and preflight behavior had moved to the signed-header contract. | green: review confirmed the focused `open_api_chat_api` coverage now exercises HMAC session, message, stream, AGUI replay, preflight, bearer rejection, stale timestamp, and replayed nonce flows. | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-frontend/packages/chat-core/src/openapi-signing.ts`
- `tron-frontend/packages/chat-openapi-adapter/src/index.ts`
- `tron-backend/src/adapters/inbound/open_api/auth.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/open_app_repository.rs`
- `tron-backend/tests/open_api_chat_api.rs`
- `tron-backend/tests/common/mod.rs`

## Commands

- `rtk make be-check`
- `rtk make be-test`
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test open_api_chat_api -- --nocapture`
- `rtk npm --prefix tron-frontend exec vitest run packages/chat-core/src/openapi-signing.test.ts packages/chat-openapi-adapter/src/index.test.ts`

## Blockers

None.
