-- A menu tree is an arrangement target. Domains own the primary navigation
-- trees; panel trees are independent targets and must not appear as domains.

CREATE TABLE IF NOT EXISTS navigation_menu_trees (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    tree_code TEXT NOT NULL UNIQUE,
    tree_type TEXT NOT NULL CHECK (tree_type IN ('Domain', 'Panel')),
    domain_id INTEGER UNIQUE REFERENCES navigation_domains(id) ON DELETE CASCADE,
    built_in INTEGER NOT NULL DEFAULT 0 CHECK (built_in IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (tree_type = 'Domain' AND domain_id IS NOT NULL)
        OR (tree_type = 'Panel' AND domain_id IS NULL)
    )
);

CREATE TABLE IF NOT EXISTS navigation_menu_tree_translations (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    tree_id INTEGER NOT NULL REFERENCES navigation_menu_trees(id) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('zh-CN', 'en-US')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    UNIQUE(tree_id, locale)
);

CREATE TABLE IF NOT EXISTS navigation_menu_tree_items (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    tree_id INTEGER NOT NULL REFERENCES navigation_menu_trees(id) ON DELETE CASCADE,
    menu_id INTEGER NOT NULL REFERENCES navigation_menus(id) ON DELETE RESTRICT,
    parent_item_id INTEGER REFERENCES navigation_menu_tree_items(id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    child_display TEXT NOT NULL DEFAULT 'Inline' CHECK (child_display IN ('Inline', 'Flyout')),
    UNIQUE(tree_id, menu_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_navigation_menu_tree_item_position
ON navigation_menu_tree_items (tree_id, COALESCE(parent_item_id, 0), position);

INSERT OR IGNORE INTO navigation_menu_trees (
    id, tree_code, tree_type, domain_id, built_in, enabled, created_at, updated_at
)
SELECT
    id, domain_code, 'Domain', id, built_in, enabled, created_at, updated_at
FROM navigation_domains
WHERE domain_code <> 'CHAT';

INSERT OR IGNORE INTO navigation_menu_trees (
    id, tree_code, tree_type, domain_id, built_in, enabled, created_at, updated_at
)
VALUES (
    734500000000000002,
    'CHAT_PANEL',
    'Panel',
    NULL,
    1,
    1,
    '2026-09-22T00:00:00Z',
    '2026-09-22T00:00:00Z'
);

INSERT OR IGNORE INTO navigation_menu_tree_translations (
    id, tree_id, locale, name, description
)
VALUES
    (734500100000000003, 734500000000000002, 'zh-CN', '会话面板', '侧边栏会话区'),
    (734500100000000004, 734500000000000002, 'en-US', 'Chat Panel', 'Sidebar chat panel');

-- Keep existing customized trees. CHAT is migrated into the panel tree.
--
-- The legacy assignment table has a self-referencing RESTRICT foreign key.
-- Deleting every legacy row in this migration fails as soon as a group has
-- children. Keep the legacy records as an inert migration source instead:
-- runtime reads only navigation_menu_trees and navigation_menu_tree_items.
INSERT OR IGNORE INTO navigation_menu_tree_items (
    id, tree_id, menu_id, parent_item_id, position
)
SELECT
    item.id,
    CASE
        WHEN domain.domain_code = 'CHAT' THEN 734500000000000002
        ELSE tree.id
    END,
    item.menu_id,
    item.parent_item_id,
    item.position
FROM navigation_domain_menu_items item
JOIN navigation_domains domain ON domain.id = item.domain_id
LEFT JOIN navigation_menu_trees tree ON tree.domain_id = domain.id
WHERE domain.domain_code = 'CHAT' OR tree.id IS NOT NULL;

INSERT OR IGNORE INTO navigation_menus (
    id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at
)
VALUES
    (734550000000000001, 'CHAT_SESSIONS', 'BuiltIn', 'Group', NULL, NULL, 'MessageSquareText', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z'),
    (734550000000000002, 'CHAT_NEW_SESSION', 'BuiltIn', 'Widget', 'chat.newSession', NULL, 'SquarePen', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z'),
    (734550000000000003, 'CHAT_PROJECTS', 'BuiltIn', 'Widget', 'chat.projects', NULL, 'FolderKanban', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z'),
    (734550000000000004, 'CHAT_RECENTS', 'BuiltIn', 'Widget', 'chat.recents', NULL, 'History', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z');

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

INSERT OR IGNORE INTO navigation_menu_tree_items (
    id, tree_id, menu_id, parent_item_id, position
)
VALUES
    (734520000000000001, 734500000000000001, 734510000000000001, NULL, 0),
    (734520000000000002, 734500000000000001, 734510000000000002, 734520000000000001, 0),
    (734520000000000003, 734500000000000001, 734510000000000003, 734520000000000001, 1),
    (734520000000000004, 734500000000000001, 734510000000000004, 734520000000000001, 2),
    (734520000000000005, 734500000000000001, 734510000000000005, NULL, 1),
    (734520000000000006, 734500000000000001, 734510000000000006, 734520000000000005, 0),
    (734520000000000007, 734500000000000001, 734510000000000007, 734520000000000005, 1),
    (734540200000000001, 734500000000000001, 734540000000000001, 734520000000000007, 0),
    (734540200000000002, 734500000000000001, 734540000000000002, 734520000000000007, 1),
    (734520000000000008, 734500000000000001, 734510000000000008, 734520000000000005, 2),
    (734520000000000009, 734500000000000001, 734510000000000009, 734520000000000005, 3),
    (734520000000000010, 734500000000000001, 734510000000000010, 734520000000000005, 4),
    (734520000000000011, 734500000000000001, 734510000000000011, 734520000000000005, 5),
    (734520000000000012, 734500000000000001, 734510000000000012, NULL, 2),
    (734520000000000013, 734500000000000001, 734510000000000013, 734520000000000012, 0),
    (734520000000000014, 734500000000000001, 734510000000000014, 734520000000000012, 1),
    (734520000000000015, 734500000000000001, 734510000000000015, 734520000000000012, 2),
    (734520000000000016, 734500000000000001, 734510000000000016, 734520000000000012, 3),
    (734520000000000017, 734500000000000001, 734510000000000017, 734520000000000012, 4),
    (734520000000000018, 734500000000000001, 734510000000000018, 734520000000000017, 0),
    (734520000000000019, 734500000000000001, 734510000000000019, 734520000000000017, 1),
    (734520000000000020, 734500000000000001, 734510000000000020, 734520000000000017, 2),
    (734520000000000021, 734500000000000001, 734510000000000021, 734520000000000017, 3),
    (734520000000000022, 734500000000000001, 734510000000000022, 734520000000000012, 5),
    (734550200000000001, 734500000000000002, 734550000000000001, NULL, 0),
    (734550200000000002, 734500000000000002, 734550000000000002, 734550200000000001, 0),
    (734550200000000003, 734500000000000002, 734550000000000003, 734550200000000001, 1),
    (734550200000000004, 734500000000000002, 734550000000000004, 734550200000000001, 2);
