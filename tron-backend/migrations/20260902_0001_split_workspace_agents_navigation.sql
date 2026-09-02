UPDATE navigation_menus
SET
    menu_type = 'Group',
    page_code = NULL,
    url = NULL,
    icon = 'Bot',
    updated_at = '2026-09-02T00:00:00Z'
WHERE menu_code = 'AGENTS' AND source = 'BuiltIn';

INSERT OR IGNORE INTO navigation_menus (
    id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at
)
VALUES
    (734540000000000001, 'INTERNAL_AGENTS', 'BuiltIn', 'Internal', 'agents.internal', NULL, 'Bot', 1, '2026-09-02T00:00:00Z', '2026-09-02T00:00:00Z'),
    (734540000000000002, 'SYSTEM_AGENTS', 'BuiltIn', 'Internal', 'agents.system', NULL, 'Server', 1, '2026-09-02T00:00:00Z', '2026-09-02T00:00:00Z');

INSERT OR IGNORE INTO navigation_menu_translations (id, menu_id, locale, label)
VALUES
    (734540100000000001, 734540000000000001, 'zh-CN', '内部智能体'),
    (734540100000000002, 734540000000000001, 'en-US', 'Internal Agents'),
    (734540100000000003, 734540000000000002, 'zh-CN', '系统智能体'),
    (734540100000000004, 734540000000000002, 'en-US', 'System Agents');

INSERT OR IGNORE INTO navigation_domain_menu_items (
    id, domain_id, menu_id, parent_item_id, position, visible_roles
)
VALUES
    (
        734540200000000001,
        734500000000000001,
        734540000000000001,
        734520000000000007,
        0,
        '[]'
    ),
    (
        734540200000000002,
        734500000000000001,
        734540000000000002,
        734520000000000007,
        1,
        '[]'
    );
