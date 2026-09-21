-- Adds the Widget menu type (rendered by a dedicated shell panel; page_code holds the
-- widget code). The chat panel tree itself is seeded by the generic menu-tree migration.

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS navigation_menus_v2 (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    menu_code TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL CHECK (source IN ('BuiltIn', 'Custom')),
    menu_type TEXT NOT NULL CHECK (menu_type IN ('Group', 'Internal', 'ExternalLink', 'Widget')),
    page_code TEXT,
    url TEXT,
    icon TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (menu_type = 'Group' AND page_code IS NULL AND url IS NULL)
        OR (menu_type = 'Internal' AND page_code IS NOT NULL AND url IS NULL)
        OR (menu_type = 'ExternalLink' AND page_code IS NULL AND url IS NOT NULL)
        OR (menu_type = 'Widget' AND page_code IS NOT NULL AND url IS NULL)
    )
);

INSERT INTO navigation_menus_v2 (
    id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at
)
SELECT
    id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at
FROM navigation_menus;

DROP TABLE navigation_menus;
ALTER TABLE navigation_menus_v2 RENAME TO navigation_menus;

PRAGMA foreign_key_check;
PRAGMA foreign_keys = ON;
