# RAG Audit Events

## Feature Summary

RAG emits safe `rag.*` audit events to the shared project event bus for write and retrieval paths. The RAG module remains a producer only: it does not consume, subscribe, render cards, or publish `agent.*` mirror events.

## Scope

- Publish write-path audit events for ingest and retry index operations.
- Publish retrieval-path audit events for search and route outcomes.
- Use the existing shared `EventBusHandle`.
- Keep event payloads limited to safe operational metadata.

## Out Of Scope

- Management, catalog, profile, corpus, diagnostics, debug, and filter-validation events.
- Event consumption, cards, AG-UI translation, or replay changes.
- New message buses, queues, channels, or sinks.

## Event Model

All events use `BusEventDraft::audit(..., "rag_service")` and keep `event_name` in the `rag.*` namespace. Publish outcomes are ignored so event bus failures cannot block write or retrieval behavior.

Allowed fields are summary metadata such as workspace, corpus, profile, source, scope, stage, route, status, counts, warning codes, error codes, trace code, and duration. Events must not include query text, document text, hit text, embeddings, raw payloads, provider payloads, or sensitive field names.
