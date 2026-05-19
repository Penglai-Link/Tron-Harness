# Database Design: Common Event Bus

## Database Decision

- base module impact: no mandatory database objects
- optional future audit sink impact: yes

The shared event-bus runtime itself should not create a common event table just to feel complete. That would waste storage and encourage indiscriminate persistence. Database storage is optional and belongs only to an audit-oriented sink that selectively persists high-value `Audit` events.

## Existing Schema Impact

- Current backend schema includes feature-owned business tables plus one specialized `user_password_reset_events` history table.
- No shared audit or internal event table exists today.
- Current repositories use SQLite connections inside service-owned adapters, so a future audit sink should use its own connection and batch writes to minimize contention.

## Proposed Schema

### Phase 1: Shared Runtime Only

- no new table
- no migration required

### Phase 2: Optional Audit Sink Outbox

If audit persistence is enabled later, create one sink-owned table such as `audit_event_outbox` with only bounded and reviewable fields:

| column | type | notes |
| --- | --- | --- |
| `id` | `INTEGER PRIMARY KEY CHECK (id > 0)` | Snowflake-backed internal row id |
| `event_id` | `INTEGER NOT NULL UNIQUE` | shared bus event id |
| `event_name` | `TEXT NOT NULL` | compact constant name |
| `producer` | `TEXT NOT NULL` | source module |
| `priority` | `TEXT NOT NULL` | `critical`, `standard`, `debug` |
| `subject_type` | `TEXT` | bounded subject type |
| `subject_code` | `TEXT` | bounded business code |
| `actor_user_code` | `TEXT` | optional actor reference |
| `request_id` | `TEXT` | optional correlation |
| `trace_id` | `TEXT` | optional correlation |
| `summary` | `TEXT` | short bounded human-readable summary |
| `attrs_json` | `TEXT` | serialized bounded attrs only |
| `payload_json` | `TEXT` | optional bounded payload for selected events |
| `occurred_at` | `INTEGER NOT NULL` | epoch millis |
| `persisted_at` | `INTEGER NOT NULL` | sink write time |
| `exported_at` | `INTEGER` | optional downstream export marker |

## Persistence Policy

- Persist only selected `Audit` events.
- Do not persist `Observe` or `Agui` by default.
- Do not persist full request bodies, tokens, passwords, prompts, or tool output dumps.
- Keep `attrs_json` and `payload_json` bounded by the same shared contract caps.
- Favor summary fields plus stable subject references over copying entire entities.

## Query And Performance Plan

- Sink writes should batch, for example 32 to 128 events per transaction.
- Sink uses a dedicated SQLite connection, not the producer repository mutex.
- Indexes should start small and practical:
  - unique index on `event_id`
  - index on `occurred_at`
  - index on `subject_code`
  - index on `actor_user_code`
- Retention and export jobs belong to the audit feature or operational tooling, not to the base shared bus.

## AGUI And Observability Storage Boundary

- AGUI replay remains in memory only, subject to hard per-stream and global caps.
- Observability sinks should emit counters, logs, or traces rather than mirror raw event rows into SQLite.
- If later requirements demand AGUI or observability persistence, those should be separate sink decisions with separate retention rules, not silent expansion of the audit table.

## Data Governance

- Secrets are forbidden in bus payloads and therefore forbidden in the audit outbox.
- Temporary passwords, raw tokens, OAuth codes, refresh tokens, cookies, and full prompt bodies must never be stored.
- Summaries should be sufficient for review without reconstructing sensitive primary payloads.
