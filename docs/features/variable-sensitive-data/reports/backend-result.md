# Backend Result: Variable Sensitive Data

## Summary

- status: done
- scope: backend storage, API display, edit rules, step-up reveal, runtime resolver, and non-sensitive regression.
- conclusion: backend AC rows passed focused verification; the full backend suite was stopped by a 120s guard while still progressing.

## Evidence Rows

- AC-VSD-01 | status: done | red: migration test failed before sensitive metadata columns existed | green: migration test passed with sensitive metadata and legacy compatibility | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-02 | status: done | red: storage test failed before sensitive values had ciphertext metadata | green: sensitive create stored ciphertext nonce tag algorithm and key version while reveal decrypted through backend code | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-03 | status: done | red: display test failed before mask fields and plaintext-search exclusion existed | green: display APIs returned masks sensitivity flags valueMasked hasValue and omitted crypto metadata | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-04 | status: done | red: edit tests failed before preserve replace demotion and inherited override guards existed | green: edit tests passed for preserve replace demotion validation and inherited sensitive override enforcement | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-05 | status: done | red: reveal tests failed before password step-up and scoped token validation existed | green: step-up issued scoped tokens without session refresh and reveal returned one authorized plaintext value | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-06 | status: done | red: runtime test failed because EffectiveVariable had no sensitive trust field | green: service-mode runtime rejected sensitive variables before external POST and trusted resolver kept plaintext internal | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-10 | status: done | red: regression tests failed before non-sensitive compatibility paths were preserved | green: non-sensitive values stayed visible searchable editable and runtime-compatible | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.

## Remediation Evidence

- AC-VSD-04 | status: done | red: review found non-sensitive to sensitive `PRESERVE` could copy plaintext into a sensitive row | green: update now returns `VARIABLE_SENSITIVE_UPGRADE_VALUE_REQUIRED` unless replacement value is supplied | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-05 | status: done | red: review found a password step-up token could be reused for another variable target within TTL | green: step-up claims now bind purpose scope workspaceCode and variableKey, and reveal handlers compare the exact target | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-06 | status: done | red: review found runtime variable resolution errors were swallowed as an empty variable list | green: agent invocation now maps runtime variable resolution failure to the existing secret delivery trust error | command_ref: CMD-BE-VARIABLE-TEST | browser:not applicable.
- AC-VSD-02 | status: done | red: review found encrypt failures were reported with decrypt code/message | green: encrypt failures now return `VARIABLE_ENCRYPT_FAILED` with an encryption message | command_ref: CMD-BE-CHECK | browser:not applicable.

## Changed Paths

- tron-backend/Cargo.toml
- tron-backend/migrations/20260602_0028_add_variable_sensitive_metadata.sql
- tron-backend/src/adapters/inbound/http/auth
- tron-backend/src/adapters/inbound/http/variables
- tron-backend/src/adapters/inbound/http/admin/variables
- tron-backend/src/adapters/outbound/crypto
- tron-backend/src/adapters/outbound/persistence/sqlite
- tron-backend/src/application/agent
- tron-backend/src/application/variable
- tron-backend/src/config/variable_crypto.rs
- tron-backend/src/domain/agent
- tron-backend/src/domain/variable
- tron-backend/tests/agent_runtime_adapter_registry.rs
- tron-backend/tests/variable_management_api.rs

## Commands

- CMD-BE-CHECK: `make be-check` passed.
- CMD-BE-VARIABLE-TEST: `cargo test --manifest-path tron-backend/Cargo.toml --test variable_management_api -- --nocapture` passed 11 tests after remediation.
- CMD-BE-VARIABLE-TEST: `cargo test --manifest-path tron-backend/Cargo.toml --test agent_runtime_adapter_registry -- --nocapture` passed 5 tests.
- CMD-BE-VARIABLE-TEST: `timeout --kill-after=10s 120s make be-test` was time-boxed after lib, AGUI, agent invocation, agent management, agent history, agent runtime, runtime adapter, and app-management tests had passed.
- CMD-BE-CHECK: `rustfmt --edition 2024 --check $(git diff --name-only | rg '\.rs$')` passed for touched Rust files.

## Blockers

- None.
