# Acceptance

## AC-1 Unified Audit Publisher

RAG event publishing uses the shared `EventBusHandle`, emits only `rag.*` events with `BusRouteClass::Audit`, applies an allowlist of safe fields, and ignores publish failures.

## AC-2 Ingest Write Events

Successful ingest emits `rag.ingest.started`, write/index stage events only for `fulltextIndex`, `vectorIndex`, `persistTree`, and `persistSource`, and a terminal `rag.ingest.completed`; ingest errors after start emit `rag.ingest.failed`.

## AC-3 Search Retrieval Events

Search emits `rag.search.started`, per-route `rag.search.route.completed` or `rag.search.route.skipped`, and a terminal `rag.search.completed` or `rag.search.failed` with safe result summaries.

## AC-4 Retry Write Events

Retry index emits `rag.retry.started`, stage completion or warning events, and a terminal `rag.retry.completed` or `rag.retry.failed`.

## AC-5 Verification Evidence

Tests prove audit route class, event names, safe metadata, failure non-blocking behavior, and absence of raw query/content/embedding/provider data.
