-- Durable queue for capability operations that must wait until every internal
-- Agent is idle. The queue and its bounded result log belong to the business
-- database so ordering, de-duplication and retention are transactional.
--
-- `operation` is one of `enable`, `disable` or `uninstall`; `force` is only
-- meaningful for uninstall and records an explicit user confirmation.

CREATE TABLE capability_queue_items (
    sequence   INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id    TEXT NOT NULL UNIQUE,
    pack_id    TEXT NOT NULL,
    pack_name  TEXT NOT NULL,
    operation  TEXT NOT NULL,
    force      INTEGER NOT NULL DEFAULT 0,
    locale     TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- One pending operation per package and kind. Enable and disable may both wait
-- so a later explicit request still wins in submission order.
CREATE UNIQUE INDEX capability_queue_items_pending_target
    ON capability_queue_items(pack_id, operation);

CREATE TABLE capability_operation_log (
    sequence    INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id     TEXT NOT NULL UNIQUE,
    pack_id     TEXT NOT NULL,
    pack_name   TEXT NOT NULL,
    operation   TEXT NOT NULL,
    force       INTEGER NOT NULL DEFAULT 0,
    succeeded   INTEGER NOT NULL,
    error       TEXT,
    error_code  TEXT,
    finished_at INTEGER NOT NULL
);

CREATE INDEX capability_operation_log_finished
    ON capability_operation_log(sequence DESC);
