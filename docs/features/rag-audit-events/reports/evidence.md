# Evidence

## RED

Command:

```sh
cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts rag_
```

Result before implementation: failed as expected.

Key failures:

- `rag_ingest_events_use_safe_payload_and_publish_failure_is_non_blocking`: existing code still emitted `rag.stage.completed`.
- `rag_search_events_use_safe_payload_and_publish_failure_is_non_blocking`: existing code still emitted `rag.route.completed`.
- `rag_retry_events_are_audit_only_and_include_terminal_failures`: retry did not emit `rag.retry.stage.completed`.

## GREEN

Command:

```sh
cargo test --manifest-path tron-backend/Cargo.toml --test rag_diagnostics_contracts rag_
```

Result after implementation:

```text
running 3 tests
test rag_ingest_events_use_safe_payload_and_publish_failure_is_non_blocking ... ok
test rag_retry_events_are_audit_only_and_include_terminal_failures ... ok
test rag_search_events_use_safe_payload_and_publish_failure_is_non_blocking ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out
```

## VERIFY

Command:

```sh
cargo fmt --manifest-path tron-backend/Cargo.toml
```

Result: passed.

Command:

```sh
make be-check
```

Result:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s)
```

Command:

```sh
make be-test
```

Result:

```text
test result: ok. 126 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
...
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
...
Doc-tests tron_backend
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Targeted source guard:

```sh
rg -n "rag\\.stage\\.completed|rag\\.route\\.completed|agent\\.tool|BusEventDraft::(observe|agui)" \
  tron-backend/src/application/rag tron-backend/tests/rag_diagnostics_contracts.rs -S
```

Result: only negative assertions remain in `rag_diagnostics_contracts.rs`; no RAG source publishes generic stage/route names, `agent.tool.*`, Observe, or AGUI events.
