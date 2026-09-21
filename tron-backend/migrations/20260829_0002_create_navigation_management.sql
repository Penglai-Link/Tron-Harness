PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS navigation_domains (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    domain_code TEXT NOT NULL UNIQUE,
    built_in INTEGER NOT NULL DEFAULT 0 CHECK (built_in IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0 CHECK (position >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS navigation_domain_translations (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    domain_id INTEGER NOT NULL REFERENCES navigation_domains(id) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('zh-CN', 'en-US')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    UNIQUE(domain_id, locale)
);

CREATE TABLE IF NOT EXISTS navigation_menus (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    menu_code TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL CHECK (source IN ('BuiltIn', 'Custom')),
    menu_type TEXT NOT NULL CHECK (menu_type IN ('Group', 'Internal', 'ExternalLink')),
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
    )
);

CREATE TABLE IF NOT EXISTS navigation_menu_translations (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    menu_id INTEGER NOT NULL REFERENCES navigation_menus(id) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('zh-CN', 'en-US')),
    label TEXT NOT NULL,
    UNIQUE(menu_id, locale)
);

CREATE TABLE IF NOT EXISTS navigation_domain_menu_items (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    domain_id INTEGER NOT NULL REFERENCES navigation_domains(id) ON DELETE CASCADE,
    menu_id INTEGER NOT NULL REFERENCES navigation_menus(id) ON DELETE RESTRICT,
    parent_item_id INTEGER REFERENCES navigation_domain_menu_items(id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    UNIQUE(domain_id, menu_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_navigation_domain_menu_item_position
ON navigation_domain_menu_items (domain_id, COALESCE(parent_item_id, 0), position);

INSERT OR IGNORE INTO navigation_domains (
    id, domain_code, built_in, enabled, is_default, position, created_at, updated_at
)
VALUES (
    734500000000000001, 'ADVANCED', 1, 1, 1, 0, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'
);

INSERT OR IGNORE INTO navigation_domain_translations (id, domain_id, locale, name, description)
VALUES
    (734500100000000001, 734500000000000001, 'zh-CN', '高级模式', '高级模式'),
    (734500100000000002, 734500000000000001, 'en-US', 'Advanced', 'Advanced');

INSERT OR IGNORE INTO navigation_menus (
    id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at
)
VALUES
    (734510000000000001, 'CORE', 'BuiltIn', 'Group', NULL, NULL, 'LayoutDashboard', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000002, 'APPS', 'BuiltIn', 'Internal', 'apps.index', NULL, 'FolderGit2', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000003, 'TASKS', 'BuiltIn', 'Internal', 'tasks.index', NULL, 'ListTodo', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000004, 'AUTOMATIONS', 'BuiltIn', 'Internal', 'automations.index', NULL, 'AlarmClock', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000005, 'AGENT_OPERATIONS', 'BuiltIn', 'Group', NULL, NULL, 'Bot', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000006, 'CHATS', 'BuiltIn', 'Internal', 'chats.index', NULL, 'MessageSquareText', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000007, 'AGENTS', 'BuiltIn', 'Internal', 'agents.index', NULL, 'Bot', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000008, 'MCP_SERVERS', 'BuiltIn', 'Internal', 'mcpServers.index', NULL, 'PlugZap', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000009, 'PROMPTS', 'BuiltIn', 'Internal', 'prompts.index', NULL, 'ScrollText', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000010, 'MODELS', 'BuiltIn', 'Internal', 'models.index', NULL, 'Sparkles', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000011, 'SKILLS', 'BuiltIn', 'Internal', 'skills.index', NULL, 'Wrench', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000012, 'DATA', 'BuiltIn', 'Group', NULL, NULL, 'Database', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000013, 'MEMORY', 'BuiltIn', 'Internal', 'memory.index', NULL, 'Database', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000014, 'KNOWLEDGE_GRAPH', 'BuiltIn', 'Internal', 'knowledgeGraph.index', NULL, 'Network', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000015, 'KNOWLEDGE_BASE', 'BuiltIn', 'Internal', 'knowledgeBase.index', NULL, 'BookOpenText', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000016, 'RAG', 'BuiltIn', 'Internal', 'rag.index', NULL, 'Search', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000017, 'RESOURCE_MANAGEMENT', 'BuiltIn', 'Group', NULL, NULL, 'Archive', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000018, 'FILE_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.files', NULL, 'Folder', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000019, 'DATA_SOURCE_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.dataSources', NULL, 'DatabaseZap', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000020, 'SERVER_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.servers', NULL, 'Server', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000021, 'SERVER_ACCOUNT_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.serverAccounts', NULL, 'UserRound', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000022, 'VARIABLES', 'BuiltIn', 'Internal', 'variables.index', NULL, 'KeyRound', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000023, 'OBSERVABILITY', 'BuiltIn', 'Internal', 'observability.index', NULL, 'Gauge', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z');

INSERT OR IGNORE INTO navigation_menu_translations (id, menu_id, locale, label)
VALUES
    (734530000000000001, 734510000000000001, 'zh-CN', '核心'),
    (734530000000000002, 734510000000000001, 'en-US', 'Core'),
    (734530000000000003, 734510000000000002, 'zh-CN', '应用'),
    (734530000000000004, 734510000000000002, 'en-US', 'Apps'),
    (734530000000000005, 734510000000000003, 'zh-CN', '任务'),
    (734530000000000006, 734510000000000003, 'en-US', 'Tasks'),
    (734530000000000007, 734510000000000004, 'zh-CN', '自动化'),
    (734530000000000008, 734510000000000004, 'en-US', 'Automations'),
    (734530000000000009, 734510000000000005, 'zh-CN', '智能体操作'),
    (734530000000000010, 734510000000000005, 'en-US', 'Agent Operations'),
    (734530000000000011, 734510000000000006, 'zh-CN', '会话'),
    (734530000000000012, 734510000000000006, 'en-US', 'Chats'),
    (734530000000000013, 734510000000000007, 'zh-CN', '智能体'),
    (734530000000000014, 734510000000000007, 'en-US', 'Agents'),
    (734530000000000015, 734510000000000008, 'zh-CN', 'MCP 服务'),
    (734530000000000016, 734510000000000008, 'en-US', 'MCP Servers'),
    (734530000000000017, 734510000000000009, 'zh-CN', '提示词'),
    (734530000000000018, 734510000000000009, 'en-US', 'Prompts'),
    (734530000000000019, 734510000000000010, 'zh-CN', '模型'),
    (734530000000000020, 734510000000000010, 'en-US', 'Models'),
    (734530000000000021, 734510000000000011, 'zh-CN', '技能'),
    (734530000000000022, 734510000000000011, 'en-US', 'Skills'),
    (734530000000000023, 734510000000000012, 'zh-CN', '数据'),
    (734530000000000024, 734510000000000012, 'en-US', 'Data'),
    (734530000000000025, 734510000000000013, 'zh-CN', '记忆'),
    (734530000000000026, 734510000000000013, 'en-US', 'Memory'),
    (734530000000000027, 734510000000000014, 'zh-CN', '知识图谱'),
    (734530000000000028, 734510000000000014, 'en-US', 'KnowledgeGraph'),
    (734530000000000029, 734510000000000015, 'zh-CN', '知识库'),
    (734530000000000030, 734510000000000015, 'en-US', 'KnowledgeBase'),
    (734530000000000031, 734510000000000016, 'zh-CN', 'RAG'),
    (734530000000000032, 734510000000000016, 'en-US', 'RAG'),
    (734530000000000033, 734510000000000017, 'zh-CN', '资源管理'),
    (734530000000000034, 734510000000000017, 'en-US', 'Resource Management'),
    (734530000000000035, 734510000000000018, 'zh-CN', '文件管理'),
    (734530000000000036, 734510000000000018, 'en-US', 'File Management'),
    (734530000000000037, 734510000000000019, 'zh-CN', '数据源管理'),
    (734530000000000038, 734510000000000019, 'en-US', 'Data Source Management'),
    (734530000000000039, 734510000000000020, 'zh-CN', '服务器管理'),
    (734530000000000040, 734510000000000020, 'en-US', 'Server Management'),
    (734530000000000041, 734510000000000021, 'zh-CN', '服务器账号管理'),
    (734530000000000042, 734510000000000021, 'en-US', 'Server Account Management'),
    (734530000000000043, 734510000000000022, 'zh-CN', '变量'),
    (734530000000000044, 734510000000000022, 'en-US', 'Variables'),
    (734530000000000045, 734510000000000023, 'zh-CN', '可观测性'),
    (734530000000000046, 734510000000000023, 'en-US', 'Observability');
