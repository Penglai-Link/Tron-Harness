CREATE TABLE IF NOT EXISTS system_initialization (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    initialized INTEGER NOT NULL DEFAULT 0 CHECK (initialized IN (0, 1)),
    initialized_at TEXT NULL,
    initialized_by TEXT NULL
);

INSERT OR IGNORE INTO system_initialization (id, initialized)
VALUES (1, 0);
