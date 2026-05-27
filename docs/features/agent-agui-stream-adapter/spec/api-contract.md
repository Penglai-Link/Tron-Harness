# API Contract: Agent AG-UI Stream Adapter

## API Overview

Protected APIs activate an AG-UI stream key, read converted replay events, and deactivate the stream.

## Endpoints

- `POST /api/v1/agents/agui/streams/{streamKey}`
  - Response: stream key and replay snapshot.
- `GET /api/v1/agents/agui/streams/{streamKey}/events`
  - Response: converted AG-UI event snapshot.
- `DELETE /api/v1/agents/agui/streams/{streamKey}`
  - Response: stream key and replay snapshot after deactivation.

## Auth And Permissions

- Existing protected-route middleware is required.
- Stream keys are opaque request identifiers in this feature.

## Error Contract

- `AGENT_AGUI_STREAM_KEY_REQUIRED`

## Frontend Backend Boundary

- No frontend AG-UI component is added.
- Existing frontend app shell must keep building and remain browser-usable.

## Database And Transaction Boundary

- No database writes or reads.
- Adapter reads replay from the existing in-memory AG-UI sink.

## Mock Or Adapter Expectations

- Tests use a local model mock and existing authenticated helpers.
- Browser verification covers authenticated app shell regression.

## Open Decisions

- None.
