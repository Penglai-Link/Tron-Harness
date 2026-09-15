-- Adds the Widget menu type (rendered by a dedicated shell panel; page_code holds the
-- widget code) and seeds the CHAT panel domain with its built-in session menu tree.

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

INSERT OR IGNORE INTO navigation_domains (
    id, domain_code, built_in, enabled, is_default, position, visible_roles, created_at, updated_at
)
VALUES (
    734500000000000002, 'CHAT', 1, 1, 0, 1, '[]', '2026-09-15T00:00:00Z', '2026-09-15T00:00:00Z'
);

INSERT OR IGNORE INTO navigation_domain_translations (id, domain_id, locale, name, description)
VALUES
    (734500100000000003, 734500000000000002, 'zh-CN', '会话面板', '侧边栏会话区'),
    (734500100000000004, 734500000000000002, 'en-US', 'Chat Panel', 'Sidebar chat panel');

INSERT OR IGNORE INTO navigation_menus (
    id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at
)
VALUES
    (734550000000000001, 'CHAT_SESSIONS', 'BuiltIn', 'Group', NULL, NULL, 'MessageSquareText', 1, '2026-09-15T00:00:00Z', '2026-09-15T00:00:00Z'),
    (734550000000000002, 'CHAT_NEW_SESSION', 'BuiltIn', 'Widget', 'chat.newSession', NULL, 'SquarePen', 1, '2026-09-15T00:00:00Z', '2026-09-15T00:00:00Z'),
    (734550000000000003, 'CHAT_PROJECTS', 'BuiltIn', 'Widget', 'chat.projects', NULL, 'FolderKanban', 1, '2026-09-15T00:00:00Z', '2026-09-15T00:00:00Z'),
    (734550000000000004, 'CHAT_RECENTS', 'BuiltIn', 'Widget', 'chat.recents', NULL, 'History', 1, '2026-09-15T00:00:00Z', '2026-09-15T00:00:00Z');

INSERT OR IGNORE INTO navigation_menu_translations (id, menu_id, locale, label)
VALUES
    (734550100000000001, 734550000000000001, 'zh-CN', '会话'),
    (734550100000000002, 734550000000000001, 'en-US', 'Chats'),
    (734550100000000003, 734550000000000002, 'zh-CN', '新会话'),
    (734550100000000004, 734550000000000002, 'en-US', 'New Chat'),
    (734550100000000005, 734550000000000003, 'zh-CN', '项目'),
    (734550100000000006, 734550000000000003, 'en-US', 'Projects'),
    (734550100000000007, 734550000000000004, 'zh-CN', '最近'),
    (734550100000000008, 734550000000000004, 'en-US', 'Recents');

INSERT OR IGNORE INTO navigation_domain_menu_items (
    id, domain_id, menu_id, parent_item_id, position, visible_roles
)
VALUES
    (734550200000000001, 734500000000000002, 734550000000000001, NULL, 0, '[]'),
    (734550200000000002, 734500000000000002, 734550000000000002, 734550200000000001, 0, '[]'),
    (734550200000000003, 734500000000000002, 734550000000000003, 734550200000000001, 1, '[]'),
    (734550200000000004, 734500000000000002, 734550000000000004, 734550200000000001, 2, '[]');
