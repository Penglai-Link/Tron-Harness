-- 全新初始化：只定义当前表结构和必要的内置目录数据，不迁移历史数据库。
-- 运行时按业务库和身份登记库分别执行对应区段。
-- Framework 自有表由 Framework 存储插件创建；动态全文和向量索引由 RAG 运行时创建。

-- tron:business

CREATE TABLE agent_loop_designs (
    loop_code TEXT PRIMARY KEY,
    version INTEGER NOT NULL,
    config_json TEXT NOT NULL
);

CREATE TABLE agent_workspace_grant_policies(agent_code TEXT PRIMARY KEY);

CREATE TABLE agent_workspace_grants(agent_code TEXT NOT NULL, workspace_code TEXT NOT NULL, PRIMARY KEY(agent_code,workspace_code));

CREATE TABLE app_tags (
    id INTEGER PRIMARY KEY,
    app_code TEXT NOT NULL REFERENCES apps(app_code),
    tag_name TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    UNIQUE (app_code, tag_name)
);

CREATE TABLE apps (
    id INTEGER PRIMARY KEY,
    app_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    app_name TEXT NOT NULL,
    description TEXT NULL,
    local_directory TEXT NOT NULL,
    runtime_role TEXT NOT NULL CHECK (runtime_role IN ('frontend', 'backend', 'mono')),
    repository_url TEXT NULL,
    default_branch TEXT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE TABLE automation_execution_records (
    id INTEGER PRIMARY KEY,
    execution_code TEXT NOT NULL UNIQUE,
    automation_code TEXT NOT NULL REFERENCES automations(automation_code),
    workspace_code TEXT NOT NULL,
    session_code TEXT NULL,
    trigger_source TEXT NOT NULL CHECK (trigger_source IN ('Scheduler', 'Manual')),
    status TEXT NOT NULL CHECK (status IN ('Pending', 'Running', 'Success', 'Failed', 'Canceled', 'Skipped')),
    summary TEXT NULL,
    scheduled_at INTEGER NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE automations (
    id INTEGER PRIMARY KEY,
    automation_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NULL,
    agent_code TEXT NOT NULL DEFAULT 'main-agent',
    model_code TEXT NULL,
    prompt_text TEXT NOT NULL,
    cron_expression TEXT NOT NULL,
    scheduler_expression TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    status TEXT NOT NULL CHECK (status IN ('Active', 'Paused', 'Archived')),
    previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Active', 'Paused')),
    last_execution_status TEXT NULL CHECK (last_execution_status IS NULL OR last_execution_status IN ('Pending', 'Running', 'Success', 'Failed', 'Canceled', 'Skipped')),
    last_execution_code TEXT NULL,
    last_execution_summary TEXT NULL,
    last_started_at INTEGER NULL,
    last_finished_at INTEGER NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE TABLE chat_session_auto_model_routes (
    workspace_code TEXT NOT NULL,
    session_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    selection_json TEXT NOT NULL,
    model_code TEXT NOT NULL,
    context_window_tokens INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, session_code, agent_code)
);

CREATE TABLE default_model_selections (
    scope_key TEXT NOT NULL,
    workspace_code TEXT,
    model_type TEXT NOT NULL,
    selection_kind TEXT NOT NULL,
    provider_code TEXT,
    model_code TEXT,
    quality_type TEXT,
    route_code TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(scope_key, model_type)
);

CREATE TABLE internal_agent_groups (
    scope_code TEXT NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 60),
    PRIMARY KEY (scope_code, name)
);

CREATE TABLE knowledge_base_derived_resources (
 document_id TEXT NOT NULL, workspace_code TEXT NOT NULL, resource_code TEXT NOT NULL,
 PRIMARY KEY(document_id,workspace_code,resource_code));

CREATE TABLE knowledge_base_documents (
 id TEXT PRIMARY KEY, knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id),
 payload_json TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0);

CREATE TABLE knowledge_base_grants (
 knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id), agent_code TEXT NOT NULL,
 PRIMARY KEY(knowledge_base_id, agent_code));

CREATE TABLE knowledge_base_ingestion_intents (
 document_id TEXT PRIMARY KEY,
 knowledge_base_id TEXT NOT NULL,
 document_revision INTEGER NOT NULL,
 retry_after_ms INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE knowledge_base_ingestion_tasks (
 id TEXT PRIMARY KEY,
 document_id TEXT NOT NULL,
 knowledge_base_id TEXT NOT NULL,
 input_config_hash TEXT NOT NULL,
 state TEXT NOT NULL,
 stage TEXT NOT NULL,
 attempt INTEGER NOT NULL,
 lease_owner TEXT,
 lease_until_ms INTEGER NOT NULL,
 retry_after_ms INTEGER NOT NULL,
 endpoint_id TEXT,
 resource_group TEXT,
 payload_json TEXT NOT NULL
);

CREATE TABLE knowledge_base_resource_privacy (
 workspace_code TEXT NOT NULL, resource_code TEXT NOT NULL, owner_user_code TEXT NOT NULL,
 PRIMARY KEY(workspace_code,resource_code));

CREATE TABLE knowledge_base_upload_requests (
 request_id TEXT PRIMARY KEY,
 content_identity TEXT NOT NULL,
 knowledge_base_id TEXT NOT NULL,
 document_id TEXT NOT NULL
);

CREATE TABLE knowledge_bases (
 id TEXT PRIMARY KEY, owner_user_code TEXT NOT NULL,
 payload_json TEXT NOT NULL, acl_revision INTEGER NOT NULL DEFAULT 0);

CREATE TABLE memory_work_domains (
    domain_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE models (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    workspace_code TEXT NULL,
    model_code TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    vendor TEXT NOT NULL,
    protocol TEXT NOT NULL,
    model_type TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_path TEXT NULL,
    api_key_ref TEXT NOT NULL,
    model_name TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    description TEXT NULL,
    thinking_enabled INTEGER NOT NULL,
    structured_output_enabled INTEGER NOT NULL,
    temperature REAL NULL,
    max_tokens INTEGER NULL,
    context_window_tokens INTEGER NULL,
    top_p REAL NULL,
    timeout_ms INTEGER NULL,
    max_retries INTEGER NULL,
    last_verified_at INTEGER NULL,
    last_verify_status TEXT NULL,
    last_verify_message TEXT NULL,
    source_kind TEXT NOT NULL,
    source_model_code TEXT NULL,
    allow_inheritance INTEGER NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
, model_capabilities TEXT NULL, responses_stateful_enabled INTEGER NOT NULL DEFAULT 0, supported_reasoning_efforts TEXT NOT NULL DEFAULT '[]', default_reasoning_effort TEXT);

CREATE TABLE navigation_domain_translations (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    domain_id INTEGER NOT NULL REFERENCES navigation_domains(id) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('zh-CN', 'en-US')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    UNIQUE(domain_id, locale)
);

CREATE TABLE navigation_domains (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    domain_code TEXT NOT NULL UNIQUE,
    built_in INTEGER NOT NULL DEFAULT 0 CHECK (built_in IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0 CHECK (position >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE navigation_menu_translations (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    menu_id INTEGER NOT NULL REFERENCES navigation_menus(id) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('zh-CN', 'en-US')),
    label TEXT NOT NULL,
    UNIQUE(menu_id, locale)
);

CREATE TABLE navigation_menu_tree_items (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    tree_id INTEGER NOT NULL REFERENCES navigation_menu_trees(id) ON DELETE CASCADE,
    menu_id INTEGER NOT NULL REFERENCES navigation_menus(id) ON DELETE RESTRICT,
    parent_item_id INTEGER REFERENCES navigation_menu_tree_items(id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    child_display TEXT NOT NULL DEFAULT 'Inline' CHECK (child_display IN ('Inline', 'Flyout')),
    UNIQUE(tree_id, menu_id)
);

CREATE TABLE navigation_menu_tree_translations (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    tree_id INTEGER NOT NULL REFERENCES navigation_menu_trees(id) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('zh-CN', 'en-US')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    UNIQUE(tree_id, locale)
);

CREATE TABLE navigation_menu_trees (
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

CREATE TABLE "navigation_menus" (
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

CREATE TABLE person_ai_device_identity (
                    singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
                    device_id TEXT NOT NULL
                );

CREATE TABLE person_ai_profile_credentials (
    person_ai_user_id TEXT PRIMARY KEY,
    person_ai_username TEXT NOT NULL,
    server_base_url TEXT NOT NULL,
    credentials_encrypted TEXT NOT NULL,
    status TEXT NOT NULL,
    authorized_at INTEGER NOT NULL,
    authorization_expires_at INTEGER,
    last_error TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE platform_default_models (
    model_type TEXT NOT NULL PRIMARY KEY,
    model_code TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE platform_discovered_tools(tool_code TEXT PRIMARY KEY, config_json TEXT NOT NULL);

CREATE TABLE platform_internal_agent_configs (
    agent_code TEXT PRIMARY KEY,
    config_json TEXT NOT NULL
);

CREATE TABLE platform_main_agent_configs (
    singleton_key TEXT PRIMARY KEY,
    personality TEXT NOT NULL DEFAULT '',
    instructions TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE platform_mcp_gateway_configs (
    singleton_key TEXT PRIMARY KEY,
    gateway_enabled INTEGER NOT NULL,
    variables_endpoint_enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE platform_mcp_servers (
    server_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    url TEXT NOT NULL,
    auth_kind TEXT NOT NULL,
    auth_payload_json TEXT,
    headers_json TEXT,
    enabled INTEGER NOT NULL,
    last_health_status TEXT NOT NULL CHECK (last_health_status IN ('never_checked', 'healthy', 'unhealthy')),
    last_health_message TEXT,
    last_health_checked_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
, transport_kind TEXT NOT NULL DEFAULT 'streamable_http', headers_variable_refs_json TEXT NOT NULL DEFAULT '{}', command TEXT NOT NULL DEFAULT '', args_json TEXT NOT NULL DEFAULT '[]', env_variable_refs_json TEXT NOT NULL DEFAULT '{}', env_passthrough_refs_json TEXT NOT NULL DEFAULT '[]', working_directory TEXT);

CREATE TABLE platform_tool_permissions(tool_code TEXT PRIMARY KEY, config_json TEXT NOT NULL);

CREATE TABLE platform_variables (
    id INTEGER PRIMARY KEY,
    variable_key TEXT NOT NULL UNIQUE,
    variable_value TEXT NOT NULL,
    description TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
, is_sensitive INTEGER NOT NULL DEFAULT 0, value_nonce TEXT NULL, value_tag TEXT NULL, value_algorithm TEXT NULL, value_key_version TEXT NULL);

CREATE TABLE project_app_links (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    project_code TEXT NOT NULL REFERENCES projects(project_code) ON DELETE CASCADE,
    app_code TEXT NOT NULL REFERENCES apps(app_code) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    is_primary INTEGER NOT NULL CHECK (is_primary IN (0, 1)),
    UNIQUE (project_code, app_code),
    UNIQUE (project_code, position)
);

CREATE TABLE projects (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    project_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'ARCHIVED')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER,
    UNIQUE (workspace_code, name)
);

CREATE TABLE prompt_parameters (
    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    parameter_name TEXT NOT NULL,
    parameter_type TEXT NOT NULL DEFAULT 'STRING'
        CHECK (parameter_type IN ('STRING', 'NUMBER', 'BOOLEAN', 'JSON')),
    description TEXT NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 200),
    position INTEGER NOT NULL,
    PRIMARY KEY (prompt_code, parameter_name)
);

CREATE TABLE prompt_translations (
    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    locale TEXT NOT NULL CHECK (locale IN ('en-US', 'zh-CN')),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) BETWEEN 1 AND 200),
    prompt_body TEXT NOT NULL CHECK (length(trim(prompt_body)) BETWEEN 1 AND 100000),
    description TEXT NULL,
    translation_status TEXT NOT NULL DEFAULT 'PUBLISHED'
        CHECK (translation_status IN ('PUBLISHED', 'DRAFT', 'UPDATE_AVAILABLE')),
    source_version INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (prompt_code, locale)
);

CREATE TABLE prompt_variables (
    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    variable_name TEXT NOT NULL,
    variable_type TEXT NOT NULL DEFAULT 'STRING'
        CHECK (variable_type IN ('STRING', 'NUMBER', 'BOOLEAN', 'JSON')),
    description TEXT NOT NULL CHECK (
        length(trim(description)) BETWEEN 1 AND 200
    ),
    position INTEGER NOT NULL,
    source_kind TEXT NOT NULL DEFAULT 'MANAGED' CHECK (source_kind IN ('MANAGED', 'RUNTIME')),
    managed_variable_key TEXT NULL,
    runtime_provider TEXT NULL,
    PRIMARY KEY (prompt_code, variable_name)
);

CREATE TABLE prompts (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('PLATFORM', 'WORKSPACE')),
    workspace_code TEXT NULL,
    prompt_code TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('SYSTEM', 'PERSONA', 'TOOL', 'TEMPLATE', 'ACTION', 'AGENT_ROLE')),
    prompt_body TEXT NOT NULL,
    description TEXT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    source_kind TEXT NOT NULL CHECK (source_kind IN (
        'PLATFORM', 'WORKSPACE_LOCAL', 'WORKSPACE_OVERRIDE', 'WORKSPACE_STATUS_PATCH'
    )),
    source_prompt_code TEXT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    allow_inheritance INTEGER NULL CHECK (allow_inheritance IN (0, 1)),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    built_in INTEGER NOT NULL DEFAULT 0 CHECK (built_in IN (0, 1)),
    runtime_required INTEGER NOT NULL DEFAULT 0 CHECK (runtime_required IN (0, 1)),
    catalog_key TEXT NULL,
    CHECK (
        (scope_kind = 'PLATFORM' AND workspace_code IS NULL AND source_kind = 'PLATFORM'
            AND source_prompt_code IS NULL AND allow_inheritance IS NULL)
        OR
        (scope_kind = 'WORKSPACE' AND workspace_code IS NOT NULL
            AND source_kind IN ('WORKSPACE_LOCAL', 'WORKSPACE_OVERRIDE', 'WORKSPACE_STATUS_PATCH'))
    ),
    CHECK (
        (source_kind = 'WORKSPACE_LOCAL' AND source_prompt_code IS NULL
            AND allow_inheritance IS NOT NULL)
        OR
        (source_kind IN ('WORKSPACE_OVERRIDE', 'WORKSPACE_STATUS_PATCH')
            AND source_prompt_code IS NOT NULL AND allow_inheritance IS NULL)
        OR source_kind = 'PLATFORM'
    )
);

CREATE TABLE provider_authorization_sessions (
    session_id TEXT PRIMARY KEY,
    provider_code TEXT NOT NULL,
    server_base_url TEXT NOT NULL,
    client_name TEXT NOT NULL,
    login_url TEXT NOT NULL,
    status TEXT NOT NULL,
    expires_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE "provider_catalog_models" (
    provider_code TEXT NOT NULL,
    model_code TEXT NOT NULL,
    model_name TEXT NOT NULL,
    upstream_vendor_code TEXT NOT NULL DEFAULT '',
    upstream_vendor_name TEXT,
    context_window_tokens INTEGER,
    protocols TEXT NOT NULL DEFAULT '["ChatCompletions"]',
    quality_type TEXT NOT NULL DEFAULT 'High',
    input_coefficient REAL,
    cache_hit_coefficient REAL,
    output_coefficient REAL,
    status TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(provider_code, model_code, upstream_vendor_code, quality_type)
);

CREATE TABLE provider_connections (
    provider_code TEXT PRIMARY KEY,
    person_ai_user_id TEXT,
    person_ai_username TEXT,
    server_base_url TEXT NOT NULL,
    status TEXT NOT NULL,
    credentials_encrypted TEXT,
    authorized_at INTEGER,
    authorization_expires_at INTEGER,
    last_sync_at INTEGER,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE provider_model_run_snapshots (
    run_code TEXT PRIMARY KEY,
    session_code TEXT,
    workspace_code TEXT NOT NULL,
    selection_kind TEXT NOT NULL,
    provider_code TEXT NOT NULL,
    requested_model_code TEXT,
    quality_type TEXT,
    route_code TEXT,
    resolved_model_code TEXT,
    resolved_model_name TEXT,
    resolved_upstream_vendor_code TEXT,
    resolved_upstream_vendor_name TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE provider_multimedia_catalog_models (
    provider_code TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_code TEXT NOT NULL,
    model_name TEXT NOT NULL,
    media_type INTEGER NOT NULL,
    status TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(provider_code, model_id)
);

CREATE TABLE provider_sync_runs (
    sync_run_code TEXT PRIMARY KEY,
    provider_code TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    model_count INTEGER,
    error_summary TEXT
);

CREATE TABLE rag_corpora (
    corpus_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('enabled', 'reserved', 'disabled')),
    description TEXT,
    capabilities_json TEXT NOT NULL,
    filter_schema_json TEXT NOT NULL,
    field_mapping_json TEXT NOT NULL,
    storage_binding_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE rag_diagnostics (
    id INTEGER PRIMARY KEY,
    record_type TEXT NOT NULL CHECK (record_type IN ('INDEXING_WARNING', 'SEARCH_TRACE')),
    record_code TEXT NOT NULL UNIQUE,
    corpus_id TEXT,
    task_id TEXT,
    source_code TEXT,
    source_version TEXT,
    scope_type TEXT,
    scope_code TEXT,
    workspace_code TEXT,
    user_code TEXT,
    agent_code TEXT,
    stage_id TEXT,
    query_text TEXT,
    message TEXT,
    payload_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER, "details_json" TEXT, "stage" TEXT, "status" TEXT, "updated_at" TEXT,
    CHECK (
        record_type <> 'SEARCH_TRACE'
        OR (
            task_id IS NOT NULL
            AND workspace_code IS NOT NULL
            AND query_text IS NOT NULL
            AND expires_at IS NOT NULL
        )
    )
);

CREATE TABLE rag_graph_evidence (
            id INTEGER PRIMARY KEY, corpus_id TEXT NOT NULL, source_code TEXT NOT NULL,
            source_version TEXT NOT NULL, scope_type TEXT NOT NULL, scope_code TEXT NOT NULL,
            target_code TEXT NOT NULL, text TEXT NOT NULL, metadata TEXT NOT NULL,
            UNIQUE(corpus_id, source_code, source_version, scope_type, scope_code, target_code)
        );

CREATE TABLE rag_memory_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_code TEXT NOT NULL,
    search_text TEXT NOT NULL DEFAULT '',
    fulltext_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    embedding_model_code TEXT,
    embedding_version TEXT,
    embedding_text_hash TEXT,
    embedding_dimension INTEGER,
    embedding BLOB,
    embedding_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
    text TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    token_count INTEGER,
    content_hash TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    source_status TEXT NOT NULL DEFAULT 'PENDING',
    title TEXT,
    source_content_hash TEXT,
    source_metadata TEXT NOT NULL DEFAULT '{}',
    source_created_at INTEGER NOT NULL DEFAULT 0,
    source_updated_at INTEGER NOT NULL DEFAULT 0,
    memory_type TEXT,
    memory_status TEXT,
    review_status TEXT,
    version TEXT,
    superseded_by TEXT,
    conflict_group TEXT,
    duplicate_group TEXT,
    injectable INTEGER NOT NULL DEFAULT 1,
    importance_score INTEGER,
    confidence_score INTEGER,
    quality_score INTEGER,
    pin_level INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER,
    derived_source_type TEXT,
    derived_source_code TEXT, "body_hash" TEXT,
    UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
);

CREATE TABLE rag_record_tasks (
            id TEXT PRIMARY KEY, user_code TEXT NOT NULL, corpus_id TEXT NOT NULL,
            state TEXT NOT NULL, payload_json TEXT NOT NULL, updated_at INTEGER NOT NULL
        );

CREATE TABLE rag_skill_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('USER', 'AGENT', 'SPACE')),
    scope_code TEXT NOT NULL,
    source_status TEXT NOT NULL CHECK (source_status IN ('PENDING', 'ACTIVE', 'FAILED', 'DISABLED', 'ARCHIVED', 'DELETED')),
    title TEXT,
    source_content_hash TEXT,
    source_metadata TEXT NOT NULL,
    source_created_at INTEGER NOT NULL,
    source_updated_at INTEGER NOT NULL,
    skill_name TEXT NOT NULL,
    skill_description TEXT NOT NULL,
    search_text TEXT NOT NULL DEFAULT '',
    fulltext_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    embedding_model_code TEXT,
    embedding_version TEXT,
    embedding_text_hash TEXT,
    embedding_dimension INTEGER,
    embedding BLOB,
    embedding_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
    text TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    token_count INTEGER,
    content_hash TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL, "body_hash" TEXT,
    UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
);

CREATE TABLE registry_skills (
    skill_name TEXT PRIMARY KEY,
    source_ref TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('Marketplace', 'GitHub', 'Git', 'Url', 'Local', 'Manual')),
    registry_path TEXT NOT NULL,
    declared_version TEXT NULL,
    compatibility TEXT NULL,
    update_status TEXT NOT NULL CHECK (update_status IN ('Unknown', 'UpToDate', 'UpdateAvailable', 'Manual', 'Error')),
    managed_by TEXT NOT NULL CHECK (managed_by IN ('Cli', 'Manual')),
    last_checked_at INTEGER NULL,
    last_updated_at INTEGER NULL,
    last_error TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE resource_chat_session_attachments (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    session_code TEXT NOT NULL COLLATE BINARY,
    attachment_code TEXT NOT NULL COLLATE BINARY,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    byte_size INTEGER NOT NULL CHECK (byte_size > 0),
    storage_locator TEXT NOT NULL COLLATE BINARY,
    created_at INTEGER NOT NULL,
    UNIQUE(session_code, attachment_code)
);

CREATE TABLE resource_code_registry (
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    kind TEXT NOT NULL,
    PRIMARY KEY (workspace_code, resource_code)
);

CREATE TABLE resource_data_source_connections (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    kind TEXT NOT NULL CHECK (kind IN ('MYSQL_CONNECTION', 'POSTGRESQL_CONNECTION', 'REDIS_CONNECTION', 'KAFKA_CONNECTION')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    attributes_json TEXT NOT NULL,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code)
);

CREATE TABLE resource_files (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    kind TEXT NOT NULL CHECK (kind IN ('FILE', 'IMAGE', 'PDF')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    file_name TEXT NOT NULL,
    file_extension TEXT NOT NULL DEFAULT '',
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    storage_locator TEXT NOT NULL COLLATE BINARY,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code)
);

CREATE TABLE resource_server_accounts (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    server_workspace_code TEXT NOT NULL COLLATE BINARY,
    server_code TEXT NOT NULL COLLATE BINARY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    username TEXT NOT NULL,
    authentication_type TEXT NOT NULL CHECK (authentication_type IN ('Password', 'PrivateKey')),
    password_variable_key TEXT,
    private_key_variable_key TEXT,
    private_key_passphrase_variable_key TEXT,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code),
    FOREIGN KEY (server_workspace_code, server_code)
        REFERENCES resource_servers (workspace_code, resource_code)
        ON DELETE RESTRICT
);

CREATE TABLE resource_servers (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port > 0 AND port <= 65535),
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code)
);

CREATE TABLE skill_rag_index_states (
    workspace_code TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    source_version TEXT NOT NULL,
    desired_enabled INTEGER NOT NULL,
    vector_status TEXT NOT NULL CHECK (vector_status IN ('PENDING', 'INDEXED', 'DISABLED', 'WARNING', 'FAILED', 'STALE')),
    graph_status TEXT NOT NULL CHECK (graph_status IN ('PENDING', 'INDEXED', 'DISABLED', 'WARNING', 'FAILED', 'STALE')),
    content_hash TEXT NULL,
    structure_hash TEXT NULL,
    last_indexed_at INTEGER NULL,
    last_error TEXT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, skill_name)
);

CREATE TABLE system_agent_registries (
    agent_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    runtime_type TEXT NOT NULL,
    command_name TEXT NOT NULL,
    source TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    available INTEGER NOT NULL DEFAULT 0,
    last_scanned_at INTEGER,
    last_seen_at INTEGER,
    last_error TEXT
);

CREATE TABLE task_agent_links (
    id INTEGER PRIMARY KEY,
    task_code TEXT NOT NULL REFERENCES tasks(task_code),
    agent_code TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    UNIQUE (task_code, agent_code)
);

CREATE TABLE task_app_links (
    id INTEGER PRIMARY KEY,
    task_code TEXT NOT NULL REFERENCES tasks(task_code),
    app_code TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    UNIQUE (task_code, app_code)
);

CREATE TABLE task_flow_statuses (
    flow_code TEXT NOT NULL REFERENCES task_flows(flow_code) ON DELETE CASCADE,
    status_code TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    required INTEGER NOT NULL DEFAULT 0 CHECK (required IN (0, 1)),
    PRIMARY KEY (flow_code, status_code),
    UNIQUE (flow_code, position)
);

CREATE TABLE task_flows (
    flow_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE "tasks" (
                id INTEGER PRIMARY KEY,
                task_code TEXT NOT NULL UNIQUE,
                workspace_code TEXT NOT NULL,
                parent_task_code TEXT NULL REFERENCES tasks(task_code),
                title TEXT NOT NULL,
                description TEXT NULL,
                execution_result TEXT NULL,
                flow_code TEXT NOT NULL,
                status TEXT NOT NULL,
                previous_status TEXT NULL,
                priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High', 'Extra High')),
                task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'Bug')),
                source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
                source_url TEXT NULL,
                agent_code TEXT NOT NULL DEFAULT 'main-agent',
                model_code TEXT NULL,
                progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                archived_at INTEGER NULL
            );

CREATE TABLE workspace_default_models (
    workspace_code TEXT NOT NULL,
    model_type TEXT NOT NULL,
    model_code TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, model_type)
);

CREATE TABLE workspace_internal_agent_definitions (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    name TEXT,
    role_prompt TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL, loop_code TEXT, avatar_id TEXT, group_name TEXT,
    PRIMARY KEY (workspace_code, agent_code)
);

CREATE TABLE workspace_internal_agent_mcp_server_assignments (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    server_code TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code, server_code),
    FOREIGN KEY (workspace_code, agent_code)
        REFERENCES workspace_internal_agent_policies (workspace_code, agent_code)
        ON DELETE CASCADE,
    FOREIGN KEY (server_code)
        REFERENCES platform_mcp_servers (server_code)
        ON DELETE CASCADE
);

CREATE TABLE workspace_internal_agent_policies (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    tool_assignment_mode TEXT NOT NULL,
    tool_exposure_mode TEXT NOT NULL,
    expanded_domain_persistence_mode TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL, max_steps INTEGER NOT NULL DEFAULT 100, max_tool_calls INTEGER NOT NULL DEFAULT 100, max_parallel_tool_calls INTEGER NOT NULL DEFAULT 4, mcp_access_mode TEXT NOT NULL DEFAULT 'DISABLED', overridden_fields TEXT NOT NULL
                 DEFAULT '["toolAssignmentMode","mcpAccessMode","toolExposureMode","expandedDomainPersistenceMode","maxSteps","maxToolCalls","maxParallelToolCalls","selectedToolCodes","selectedServerCodes"]',
    PRIMARY KEY (workspace_code, agent_code)
);

CREATE TABLE workspace_internal_agent_tool_assignments (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    tool_code TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code, tool_code)
);

CREATE TABLE workspace_mcp_server_policies (
    workspace_code TEXT NOT NULL,
    server_code TEXT NOT NULL REFERENCES platform_mcp_servers(server_code) ON DELETE CASCADE,
    enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, server_code)
);

CREATE TABLE workspace_session_links (session_id TEXT PRIMARY KEY, workspace_code TEXT NOT NULL, business_json TEXT NOT NULL);

CREATE TABLE workspace_skill_bindings (
    workspace_code TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, skill_name)
);

CREATE TABLE workspace_system_agent_policies (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code)
);

CREATE TABLE workspace_task_flows (
    workspace_code TEXT PRIMARY KEY,
    flow_code TEXT NOT NULL REFERENCES task_flows(flow_code),
    updated_at INTEGER NOT NULL
);

CREATE TABLE workspace_variables (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL,
    variable_key TEXT NOT NULL,
    variable_value TEXT NOT NULL,
    description TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL, is_sensitive INTEGER NOT NULL DEFAULT 0, value_nonce TEXT NULL, value_tag TEXT NULL, value_algorithm TEXT NULL, value_key_version TEXT NULL,
    UNIQUE (workspace_code, variable_key)
);

CREATE TABLE workspaces (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NULL,
    parent_workspace_code TEXT NULL REFERENCES workspaces(workspace_code),
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE INDEX idx_app_tags_app_code ON app_tags(app_code);

CREATE INDEX idx_apps_workspace_name ON apps(workspace_code, app_name);

CREATE INDEX idx_apps_workspace_status ON apps(workspace_code, status);

CREATE INDEX idx_automation_executions_automation_started ON automation_execution_records(automation_code, started_at DESC);

CREATE INDEX idx_automation_executions_session ON automation_execution_records(session_code);

CREATE INDEX idx_automation_executions_workspace_started ON automation_execution_records(workspace_code, started_at DESC);

CREATE UNIQUE INDEX idx_automation_scheduler_tick ON automation_execution_records(automation_code, trigger_source, scheduled_at)
WHERE trigger_source = 'Scheduler' AND scheduled_at IS NOT NULL;

CREATE INDEX idx_automations_workspace_status ON automations(workspace_code, status);

CREATE INDEX idx_automations_workspace_updated ON automations(workspace_code, updated_at);

CREATE INDEX idx_chat_session_auto_model_routes_session
    ON chat_session_auto_model_routes(session_code);

CREATE INDEX idx_kb_derived_resource_reference
 ON knowledge_base_derived_resources(workspace_code,resource_code);

CREATE INDEX idx_kb_documents_parent ON knowledge_base_documents(knowledge_base_id);

CREATE INDEX idx_kb_ingestion_claim
 ON knowledge_base_ingestion_tasks(state,stage,retry_after_ms,lease_until_ms);

CREATE UNIQUE INDEX idx_kb_ingestion_one_live_document
 ON knowledge_base_ingestion_tasks(document_id)
 WHERE state IN ('QUEUED', 'RUNNING');

CREATE INDEX idx_knowledge_bases_owner ON knowledge_bases(owner_user_code);

CREATE INDEX idx_memory_work_domains_status
    ON memory_work_domains (status, name);

CREATE UNIQUE INDEX idx_models_platform_display_name
    ON models(display_name)
    WHERE scope_kind = 'platform';

CREATE INDEX idx_models_scope_workspace
    ON models(scope_kind, workspace_code);

CREATE INDEX idx_models_source_model_code
    ON models(source_model_code);

CREATE UNIQUE INDEX idx_models_workspace_display_name
    ON models(workspace_code, display_name)
    WHERE scope_kind = 'workspace';

CREATE UNIQUE INDEX idx_navigation_menu_tree_item_position
ON navigation_menu_tree_items (tree_id, COALESCE(parent_item_id, 0), position);

CREATE INDEX idx_person_ai_profile_credentials_status
    ON person_ai_profile_credentials(status, updated_at DESC);

CREATE INDEX idx_platform_default_models_model_code
    ON platform_default_models(model_code);

CREATE INDEX idx_platform_mcp_servers_enabled_updated_at
    ON platform_mcp_servers (enabled, updated_at);

CREATE INDEX idx_platform_variables_variable_key
    ON platform_variables(variable_key);

CREATE UNIQUE INDEX idx_project_app_links_primary
    ON project_app_links(project_code)
    WHERE is_primary = 1;

CREATE INDEX idx_projects_workspace_updated
    ON projects(workspace_code, status, updated_at DESC, id DESC);

CREATE INDEX idx_prompt_parameters_prompt_position
    ON prompt_parameters(prompt_code, position);

CREATE INDEX idx_prompt_translations_locale
    ON prompt_translations(locale, prompt_code);

CREATE INDEX idx_prompt_variables_prompt_position
    ON prompt_variables(prompt_code, position);

CREATE UNIQUE INDEX idx_prompts_catalog_key
    ON prompts(catalog_key)
    WHERE catalog_key IS NOT NULL;

CREATE UNIQUE INDEX idx_prompts_platform_display_name
    ON prompts(display_name)
    WHERE scope_kind = 'PLATFORM';

CREATE INDEX idx_prompts_scope_workspace
    ON prompts(scope_kind, workspace_code);

CREATE UNIQUE INDEX idx_prompts_workspace_definition_display_name
    ON prompts(workspace_code, display_name)
    WHERE source_kind IN ('WORKSPACE_LOCAL', 'WORKSPACE_OVERRIDE');

CREATE UNIQUE INDEX idx_prompts_workspace_source_slot
    ON prompts(workspace_code, source_prompt_code)
    WHERE source_prompt_code IS NOT NULL;

CREATE INDEX idx_provider_authorization_sessions_provider_updated
    ON provider_authorization_sessions(provider_code, updated_at DESC);

CREATE INDEX idx_provider_catalog_models_provider_status
    ON provider_catalog_models(provider_code, status, model_name);

CREATE INDEX idx_provider_multimedia_catalog_models_provider_status
    ON provider_multimedia_catalog_models(provider_code, status, media_type, model_name);

CREATE INDEX idx_provider_run_snapshots_session
    ON provider_model_run_snapshots(session_code);

CREATE INDEX idx_provider_sync_runs_provider_started
    ON provider_sync_runs(provider_code, started_at DESC);

CREATE INDEX "idx_rag_diagnostics_source_scope"
                ON "rag_diagnostics" (source_code, source_version, scope_type, scope_code, stage);

CREATE INDEX idx_rag_graph_evidence_source ON rag_graph_evidence(corpus_id, source_code, scope_type, scope_code);

CREATE INDEX "idx_rag_memory_text_units_source_scope"
                ON "rag_memory_text_units" (source_code, source_version, scope_type, scope_code, ordinal);

CREATE INDEX idx_rag_record_tasks_owner ON rag_record_tasks(user_code, updated_at);

CREATE INDEX "idx_rag_skill_text_units_source_scope"
                ON "rag_skill_text_units" (source_code, source_version, scope_type, scope_code, ordinal);

CREATE INDEX idx_registry_skills_managed_by
    ON registry_skills(managed_by);

CREATE INDEX idx_registry_skills_source_type
    ON registry_skills(source_type);

CREATE INDEX idx_resource_chat_session_attachments_session
    ON resource_chat_session_attachments (session_code, created_at, id);

CREATE INDEX idx_resource_chat_session_attachments_workspace
    ON resource_chat_session_attachments (workspace_code);

CREATE INDEX idx_resource_data_source_connections_workspace
    ON resource_data_source_connections (workspace_code);

CREATE INDEX idx_resource_files_workspace
    ON resource_files (workspace_code);

CREATE INDEX idx_resource_server_accounts_server
    ON resource_server_accounts (workspace_code, server_code);

CREATE INDEX idx_resource_server_accounts_workspace
    ON resource_server_accounts (workspace_code);

CREATE INDEX idx_resource_servers_workspace
    ON resource_servers (workspace_code);

CREATE INDEX idx_skill_rag_index_states_status
    ON skill_rag_index_states(workspace_code, vector_status, graph_status);

CREATE INDEX idx_task_agent_links_task_code ON task_agent_links(task_code);

CREATE INDEX idx_task_app_links_task_code ON task_app_links(task_code);

CREATE INDEX idx_task_flow_statuses_flow_position
    ON task_flow_statuses(flow_code, position);

CREATE INDEX idx_tasks_parent_task_code ON tasks(parent_task_code);

CREATE INDEX idx_tasks_workspace_flow ON tasks(workspace_code, flow_code);

CREATE INDEX idx_tasks_workspace_status ON tasks(workspace_code, status);

CREATE INDEX idx_workspace_default_models_model_code
    ON workspace_default_models(model_code);

CREATE INDEX idx_workspace_internal_agent_mcp_server_assignments_workspace
    ON workspace_internal_agent_mcp_server_assignments (workspace_code, agent_code);

CREATE INDEX idx_workspace_internal_agent_policies_workspace_code
    ON workspace_internal_agent_policies (workspace_code);

CREATE INDEX idx_workspace_internal_agent_tool_assignments_agent_code
    ON workspace_internal_agent_tool_assignments (agent_code);

CREATE INDEX idx_workspace_internal_agent_tool_assignments_workspace_code
    ON workspace_internal_agent_tool_assignments (workspace_code);

CREATE INDEX idx_workspace_mcp_server_policies_workspace_code
    ON workspace_mcp_server_policies (workspace_code);

CREATE INDEX idx_workspace_skill_bindings_skill
    ON workspace_skill_bindings(skill_name);

CREATE INDEX idx_workspace_skill_bindings_workspace
    ON workspace_skill_bindings(workspace_code);

CREATE INDEX idx_workspace_system_agent_policies_workspace_code
    ON workspace_system_agent_policies (workspace_code);

CREATE INDEX idx_workspace_variables_variable_key
    ON workspace_variables(variable_key);

CREATE INDEX idx_workspace_variables_workspace_code
    ON workspace_variables(workspace_code);

CREATE INDEX idx_workspaces_parent_workspace_code
    ON workspaces(parent_workspace_code);

CREATE INDEX idx_workspaces_status
    ON workspaces(status);

CREATE INDEX workspace_session_links_workspace ON workspace_session_links(workspace_code);

CREATE TRIGGER platform_agent_loop_insert BEFORE INSERT ON platform_internal_agent_configs
WHEN COALESCE(json_extract(NEW.config_json, '$.loopCode'), '') != ''
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = json_extract(NEW.config_json, '$.loopCode'))
    THEN RAISE(ABORT, 'unknown agent loop') END; END;

CREATE TRIGGER platform_agent_loop_update BEFORE UPDATE OF config_json ON platform_internal_agent_configs
WHEN COALESCE(json_extract(NEW.config_json, '$.loopCode'), '') != ''
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = json_extract(NEW.config_json, '$.loopCode'))
    THEN RAISE(ABORT, 'unknown agent loop') END; END;

CREATE TRIGGER resource_server_accounts_require_parent_insert
             BEFORE INSERT ON resource_server_accounts
             FOR EACH ROW
             WHEN NEW.server_workspace_code IS NULL
                  OR NOT EXISTS (
                      SELECT 1 FROM resource_servers
                      WHERE workspace_code = NEW.server_workspace_code
                        AND resource_code = NEW.server_code
                  )
             BEGIN
                 SELECT RAISE(ABORT, 'resource server account parent does not exist');
             END;

CREATE TRIGGER resource_server_accounts_require_parent_update
             BEFORE UPDATE OF server_workspace_code, server_code ON resource_server_accounts
             FOR EACH ROW
             WHEN NEW.server_workspace_code IS NULL
                  OR NOT EXISTS (
                      SELECT 1 FROM resource_servers
                      WHERE workspace_code = NEW.server_workspace_code
                        AND resource_code = NEW.server_code
                  )
             BEGIN
                 SELECT RAISE(ABORT, 'resource server account parent does not exist');
             END;

CREATE TRIGGER resource_servers_prevent_parent_delete
             BEFORE DELETE ON resource_servers
             FOR EACH ROW
             WHEN EXISTS (
                 SELECT 1 FROM resource_server_accounts
                 WHERE server_workspace_code = OLD.workspace_code
                   AND server_code = OLD.resource_code
             )
             BEGIN
                 SELECT RAISE(ABORT, 'resource server has accounts');
             END;

CREATE TRIGGER trg_prompts_immutable_identity
BEFORE UPDATE ON prompts
FOR EACH ROW
WHEN NEW.prompt_code <> OLD.prompt_code
    OR NEW.category <> OLD.category
    OR NEW.scope_kind <> OLD.scope_kind
    OR COALESCE(NEW.workspace_code, '') <> COALESCE(OLD.workspace_code, '')
BEGIN
    SELECT RAISE(ABORT, 'prompt identity fields are immutable');
END;

CREATE TRIGGER workspace_agent_loop_insert BEFORE INSERT ON workspace_internal_agent_definitions
WHEN NEW.loop_code IS NOT NULL
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = NEW.loop_code)
    THEN RAISE(ABORT, 'unknown agent loop') END; END;

CREATE TRIGGER workspace_agent_loop_update BEFORE UPDATE OF loop_code ON workspace_internal_agent_definitions
WHEN NEW.loop_code IS NOT NULL
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = NEW.loop_code)
    THEN RAISE(ABORT, 'unknown agent loop') END; END;

-- 内置目录数据

-- memory_work_domains
INSERT INTO memory_work_domains (domain_code, name, description, status, created_at, updated_at) VALUES
    ('main-domain', 'main-domain', 'Default work domain used by existing agent memory tools.', 'ACTIVE', 1781568000000, 1781568000000),
    ('frontend', 'frontend', 'Frontend product and interface implementation knowledge.', 'ACTIVE', 1781568000000, 1781568000000),
    ('backend', 'backend', 'Backend service, storage, and API implementation knowledge.', 'ACTIVE', 1781568000000, 1781568000000),
    ('testing', 'testing', 'Testing, validation, and quality assurance knowledge.', 'ACTIVE', 1781568000000, 1781568000000),
    ('product', 'product', 'Product design, requirements, and workflow knowledge.', 'ACTIVE', 1781568000000, 1781568000000);

-- navigation_domains
INSERT INTO navigation_domains (id, domain_code, built_in, enabled, is_default, position, created_at, updated_at) VALUES
    (734500000000000001, 'ADVANCED', 1, 1, 1, 0, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z');

-- navigation_domain_translations
INSERT INTO navigation_domain_translations (id, domain_id, locale, name, description) VALUES
    (734500100000000001, 734500000000000001, 'zh-CN', '高级模式', '高级模式'),
    (734500100000000002, 734500000000000001, 'en-US', 'Advanced', 'Advanced');

-- navigation_menus
INSERT INTO navigation_menus (id, menu_code, source, menu_type, page_code, url, icon, enabled, created_at, updated_at) VALUES
    (734510000000000001, 'CORE', 'BuiltIn', 'Group', NULL, NULL, 'LayoutDashboard', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000002, 'APPS', 'BuiltIn', 'Internal', 'apps.index', NULL, 'FolderGit2', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000003, 'TASKS', 'BuiltIn', 'Internal', 'tasks.index', NULL, 'ListTodo', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000004, 'AUTOMATIONS', 'BuiltIn', 'Internal', 'automations.index', NULL, 'AlarmClock', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000005, 'AGENT_OPERATIONS', 'BuiltIn', 'Group', NULL, NULL, 'Bot', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000008, 'MCP_SERVERS', 'BuiltIn', 'Internal', 'mcpServers.index', NULL, 'PlugZap', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000009, 'PROMPTS', 'BuiltIn', 'Internal', 'prompts.index', NULL, 'ScrollText', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000010, 'MODELS', 'BuiltIn', 'Internal', 'models.index', NULL, 'Sparkles', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000011, 'SKILLS', 'BuiltIn', 'Internal', 'skills.index', NULL, 'Wrench', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000012, 'DATA', 'BuiltIn', 'Group', NULL, NULL, 'Database', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000017, 'RESOURCE_MANAGEMENT', 'BuiltIn', 'Group', NULL, NULL, 'Archive', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000018, 'FILE_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.files', NULL, 'Folder', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000019, 'DATA_SOURCE_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.dataSources', NULL, 'DatabaseZap', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000020, 'SERVER_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.servers', NULL, 'Server', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000021, 'SERVER_ACCOUNT_MANAGEMENT', 'BuiltIn', 'Internal', 'resourceManagement.serverAccounts', NULL, 'UserRound', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000022, 'VARIABLES', 'BuiltIn', 'Internal', 'variables.index', NULL, 'KeyRound', 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734510000000000030, 'OBSERVABILITY', 'BuiltIn', 'Internal', 'observability.index', NULL, 'Activity', 1, '2026-10-10T00:00:00Z', '2026-10-10T00:00:00Z'),
    (734550000000000001, 'CHAT_SESSIONS', 'BuiltIn', 'Group', NULL, NULL, 'MessageSquareText', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z'),
    (734550000000000002, 'CHAT_NEW_SESSION', 'BuiltIn', 'Widget', 'chat.newSession', NULL, 'SquarePen', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z'),
    (734550000000000003, 'CHAT_PROJECTS', 'BuiltIn', 'Widget', 'chat.projects', NULL, 'FolderKanban', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z'),
    (734550000000000004, 'CHAT_RECENTS', 'BuiltIn', 'Widget', 'chat.recents', NULL, 'History', 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z');

-- navigation_menu_translations
INSERT INTO navigation_menu_translations (id, menu_id, locale, label) VALUES
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
    (734530000000000060, 734510000000000030, 'zh-CN', '可观测'),
    (734530000000000061, 734510000000000030, 'en-US', 'Observability'),
    (734550100000000001, 734550000000000001, 'zh-CN', '会话'),
    (734550100000000002, 734550000000000001, 'en-US', 'Chats'),
    (734550100000000003, 734550000000000002, 'zh-CN', '新会话'),
    (734550100000000004, 734550000000000002, 'en-US', 'New Chat'),
    (734550100000000005, 734550000000000003, 'zh-CN', '项目'),
    (734550100000000006, 734550000000000003, 'en-US', 'Projects'),
    (734550100000000007, 734550000000000004, 'zh-CN', '最近'),
    (734550100000000008, 734550000000000004, 'en-US', 'Recents');

-- navigation_menu_trees
INSERT INTO navigation_menu_trees (id, tree_code, tree_type, domain_id, built_in, enabled, created_at, updated_at) VALUES
    (734500000000000001, 'ADVANCED', 'Domain', 734500000000000001, 1, 1, '2026-08-29T00:00:00Z', '2026-08-29T00:00:00Z'),
    (734500000000000002, 'CHAT_PANEL', 'Panel', NULL, 1, 1, '2026-09-22T00:00:00Z', '2026-09-22T00:00:00Z');

-- navigation_menu_tree_translations
INSERT INTO navigation_menu_tree_translations (id, tree_id, locale, name, description) VALUES
    (734500100000000003, 734500000000000002, 'zh-CN', '会话面板', '侧边栏会话区'),
    (734500100000000004, 734500000000000002, 'en-US', 'Chat Panel', 'Sidebar chat panel');

-- navigation_menu_tree_items
INSERT INTO navigation_menu_tree_items (id, tree_id, menu_id, parent_item_id, position, child_display) VALUES
    (734520000000000001, 734500000000000001, 734510000000000001, NULL, 0, 'Flyout'),
    (734520000000000002, 734500000000000001, 734510000000000002, 734520000000000001, 0, 'Inline'),
    (734520000000000003, 734500000000000001, 734510000000000003, 734520000000000001, 1, 'Inline'),
    (734520000000000004, 734500000000000001, 734510000000000004, 734520000000000001, 2, 'Inline'),
    (734520000000000005, 734500000000000001, 734510000000000005, NULL, 1, 'Flyout'),
    (734520000000000008, 734500000000000001, 734510000000000008, 734520000000000005, 1, 'Inline'),
    (734520000000000009, 734500000000000001, 734510000000000009, 734520000000000005, 2, 'Inline'),
    (734520000000000010, 734500000000000001, 734510000000000010, 734520000000000005, 3, 'Inline'),
    (734520000000000011, 734500000000000001, 734510000000000011, 734520000000000005, 4, 'Inline'),
    (734520000000000012, 734500000000000001, 734510000000000012, NULL, 2, 'Flyout'),
    (734520000000000017, 734500000000000001, 734510000000000017, 734520000000000012, 4, 'Inline'),
    (734520000000000018, 734500000000000001, 734510000000000018, 734520000000000017, 0, 'Inline'),
    (734520000000000019, 734500000000000001, 734510000000000019, 734520000000000017, 1, 'Inline'),
    (734520000000000020, 734500000000000001, 734510000000000020, 734520000000000017, 2, 'Inline'),
    (734520000000000021, 734500000000000001, 734510000000000021, 734520000000000017, 3, 'Inline'),
    (734520000000000022, 734500000000000001, 734510000000000022, 734520000000000012, 5, 'Inline'),
    (734550000000000030, 734500000000000001, 734510000000000030, NULL, 3, 'Inline'),
    (734550200000000001, 734500000000000002, 734550000000000001, NULL, 0, 'Inline'),
    (734550200000000002, 734500000000000002, 734550000000000002, 734550200000000001, 0, 'Inline'),
    (734550200000000003, 734500000000000002, 734550000000000003, 734550200000000001, 1, 'Inline'),
    (734550200000000004, 734500000000000002, 734550000000000004, 734550200000000001, 2, 'Inline');

-- task_flows
INSERT INTO task_flows (flow_code, name, created_at, updated_at) VALUES
    ('DEFAULT', '默认流程', CAST(strftime('%s', 'now') AS INTEGER) * 1000, CAST(strftime('%s', 'now') AS INTEGER) * 1000);

-- task_flow_statuses
INSERT INTO task_flow_statuses (flow_code, status_code, name, position, required) VALUES
    ('DEFAULT', 'NOT_STARTED', '未开始', 0, 1),
    ('DEFAULT', 'IN_PROGRESS', '进行中', 1, 0),
    ('DEFAULT', 'REOPEN', '重新打开', 2, 0),
    ('DEFAULT', 'CLOSED', '关闭', 3, 1);

-- rag_corpora
INSERT INTO rag_corpora (corpus_id, display_name, status, description, capabilities_json, filter_schema_json, field_mapping_json, storage_binding_json, created_at, updated_at) VALUES
    ('memory-card', '记忆', 'enabled', 'User, agent, and space scoped memory cards.', '{"fulltext":true,"vector":true,"metadataGroups":[{"name":"memory","fields":["memoryType","memoryStatus","reviewStatus","version","supersededBy","conflictGroup","duplicateGroup","injectable","importanceScore","confidenceScore","qualityScore","pinLevel","expiresAt","derivedSourceType","derivedSourceCode"]}],"settings":{"fulltext":true,"vector":true,"graph":false,"embeddingModelCode":null,"rerankModelCode":null,"graphModelCode":null},"graph":false}', '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"source_status"},"sourceStatus":{"type":"string","column":"source_status"},"textUnitStatus":{"type":"string","column":"status"},"memoryType":{"type":"string","column":"memory_type"},"memoryStatus":{"type":"string","column":"memory_status"},"reviewStatus":{"type":"string","column":"review_status"},"version":{"type":"string","column":"version"},"supersededBy":{"type":"string","column":"superseded_by"},"conflictGroup":{"type":"string","column":"conflict_group"},"duplicateGroup":{"type":"string","column":"duplicate_group"},"injectable":{"type":"boolean","column":"injectable"},"importanceScore":{"type":"integer","column":"importance_score","operators":["eq","in","gt","gte","lt","lte"]},"confidenceScore":{"type":"integer","column":"confidence_score","operators":["eq","in","gt","gte","lt","lte"]},"qualityScore":{"type":"integer","column":"quality_score","operators":["eq","in","gt","gte","lt","lte"]},"pinLevel":{"type":"integer","column":"pin_level","operators":["eq","in","gt","gte","lt","lte"]},"expiresAt":{"type":"integer","column":"expires_at","operators":["eq","in","gt","gte","lt","lte","gtOrNull"]},"derivedSourceType":{"type":"string","column":"derived_source_type"},"derivedSourceCode":{"type":"string","column":"derived_source_code"}}}', '{"source":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","status":"source_status","sourceStatus":"source_status","title":"title","sourceContentHash":"source_content_hash","sourceMetadata":"source_metadata","memoryType":"memory_type","memoryStatus":"memory_status","reviewStatus":"review_status","version":"version","supersededBy":"superseded_by","conflictGroup":"conflict_group","duplicateGroup":"duplicate_group","injectable":"injectable","importanceScore":"importance_score","confidenceScore":"confidence_score","qualityScore":"quality_score","pinLevel":"pin_level","expiresAt":"expires_at","derivedSourceType":"derived_source_type","derivedSourceCode":"derived_source_code","sourceCreatedAt":"source_created_at","sourceUpdatedAt":"source_updated_at"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","textUnitStatus":"status","text":"text"}}', '{"mode":"TARGETED","bindings":{"source":"memory-store","textUnit":"memory-store","fulltext":"memory-store","vector":"memory-store","diagnostics":"diagnostics-store"},"targets":[{"id":"memory-store","backend":"SQLITE","layout":"TABLE","table":"rag_memory_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}', 1772294400000, 1781568000000),
    ('skill', '技能', 'enabled', 'Workspace-scoped skill content and source-linked graph evidence.', '{"fulltext":true,"vector":true,"metadataGroups":[{"name":"skill","fields":["skillName","skillDescription"]}],"settings":{"fulltext":true,"vector":true,"graph":true,"embeddingModelCode":null,"rerankModelCode":null,"graphModelCode":null},"graph":true}', '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"source_status"},"sourceStatus":{"type":"string","column":"source_status"},"textUnitStatus":{"type":"string","column":"status"},"skillName":{"type":"string","column":"skill_name"}}}', '{"source":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","status":"source_status","sourceStatus":"source_status","title":"title","sourceContentHash":"source_content_hash","sourceMetadata":"source_metadata","skillName":"skill_name","skillDescription":"skill_description"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","textUnitStatus":"status","text":"text"}}', '{"mode":"TARGETED","bindings":{"source":"skill-store","textUnit":"skill-store","fulltext":"skill-store","vector":"skill-store","diagnostics":"diagnostics-store"},"targets":[{"id":"skill-store","backend":"SQLITE","layout":"TABLE","table":"rag_skill_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}', 1780339200000, 1780339200000);

CREATE TABLE IF NOT EXISTS tron_schema_initialized (scope TEXT PRIMARY KEY);
INSERT INTO tron_schema_initialized (scope) VALUES ('business');

-- tron:profiles

CREATE TABLE profile_avatar_cache (
    person_ai_user_id TEXT PRIMARY KEY NOT NULL,
    content_type TEXT NOT NULL,
    image_bytes BLOB NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (person_ai_user_id) REFERENCES profiles(person_ai_user_id) ON DELETE CASCADE
);

CREATE TABLE profiles (
    profile_id TEXT PRIMARY KEY NOT NULL,
    person_ai_user_id TEXT NOT NULL DEFAULT '' UNIQUE,
    person_ai_username TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TEXT NOT NULL,
    last_activated_at TEXT
);

CREATE TABLE IF NOT EXISTS tron_schema_initialized (scope TEXT PRIMARY KEY);
INSERT INTO tron_schema_initialized (scope) VALUES ('profiles');
