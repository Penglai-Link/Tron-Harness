-- Rename the built-in Dude / 好兄弟 assistant domain to Assistant / PersonAI.
-- 0001_init.sql still seeds the original identifiers, menu labels and default
-- name, so this migration also covers databases created from scratch.
-- User-customized assistant names, menu labels and menu descriptions are kept.

PRAGMA legacy_alter_table = OFF;

-- 1. Tables.
ALTER TABLE dudes RENAME TO assistants;
ALTER TABLE dude_souls RENAME TO assistant_souls;
ALTER TABLE dude_sessions RENAME TO assistant_sessions;
ALTER TABLE dude_events RENAME TO assistant_events;
ALTER TABLE dude_memory_batches RENAME TO assistant_memory_batches;
ALTER TABLE dude_memory_cursors RENAME TO assistant_memory_cursors;
ALTER TABLE dude_skill_candidates RENAME TO assistant_skill_candidates;
ALTER TABLE dude_work_changes RENAME TO assistant_work_changes;

-- 2. Columns.
ALTER TABLE assistants RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_souls RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_sessions RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_events RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_memory_batches RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_memory_cursors RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_skill_candidates RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE assistant_work_changes RENAME COLUMN dude_code TO assistant_code;
ALTER TABLE tasks RENAME COLUMN owner_dude_code TO owner_assistant_code;

-- 3. Index names.
DROP INDEX IF EXISTS dudes_single_main;
CREATE UNIQUE INDEX assistants_single_main ON assistants(is_main) WHERE is_main=1;
DROP INDEX IF EXISTS dude_events_ready;
CREATE INDEX assistant_events_ready ON assistant_events(status,available_at,lease_until);
DROP INDEX IF EXISTS dude_events_dude;
CREATE INDEX assistant_events_assistant ON assistant_events(assistant_code,source,status);
DROP INDEX IF EXISTS dude_events_session;
CREATE INDEX assistant_events_session ON assistant_events(session_code,status);
DROP INDEX IF EXISTS dude_memory_one_active;
CREATE UNIQUE INDEX assistant_memory_one_active ON assistant_memory_batches(user_code,assistant_code) WHERE status='Active';
DROP INDEX IF EXISTS dude_skill_candidates_user;
CREATE INDEX assistant_skill_candidates_user ON assistant_skill_candidates(user_code,created_at);
DROP INDEX IF EXISTS idx_dude_work_changes_task;
CREATE INDEX idx_assistant_work_changes_task ON assistant_work_changes(assistant_code, task_code, created_at);
DROP INDEX IF EXISTS idx_tasks_dude_followup;
CREATE INDEX idx_tasks_assistant_followup ON tasks(owner_assistant_code, next_followup_at) WHERE owner_assistant_code IS NOT NULL AND archived_at IS NULL;

-- 4. Identity triggers.
DROP TRIGGER IF EXISTS dude_config_name_insert;
DROP TRIGGER IF EXISTS dude_config_name_update;
CREATE TRIGGER assistant_config_name_insert AFTER INSERT ON platform_internal_agent_configs
WHEN EXISTS(SELECT 1 FROM assistant_souls WHERE assistant_code=NEW.agent_code)
BEGIN UPDATE platform_internal_agent_configs SET config_json=json_remove(NEW.config_json,'$.name') WHERE agent_code=NEW.agent_code; END;
CREATE TRIGGER assistant_config_name_update AFTER UPDATE OF config_json ON platform_internal_agent_configs
WHEN EXISTS(SELECT 1 FROM assistant_souls WHERE assistant_code=NEW.agent_code) AND json_type(NEW.config_json,'$.name') IS NOT NULL
BEGIN UPDATE platform_internal_agent_configs SET config_json=json_remove(NEW.config_json,'$.name') WHERE agent_code=NEW.agent_code; END;

-- 5. Built-in assistant codes. Insert the renamed parent rows first so the
--    sessions foreign key stays valid even when foreign keys are enforced.
INSERT INTO assistants (assistant_code, record_json, enabled, heartbeat_minutes, next_heartbeat_at, is_main)
SELECT CASE assistant_code
           WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
       END,
       replace(replace(replace(record_json,'"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant'),
       enabled, heartbeat_minutes, next_heartbeat_at, 0
FROM assistants
WHERE assistant_code IN ('main-dude','memory-dude');

UPDATE assistant_sessions SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
UPDATE assistant_events SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
UPDATE assistant_memory_batches SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
UPDATE assistant_memory_cursors SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
UPDATE assistant_skill_candidates SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
UPDATE assistant_work_changes SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
UPDATE tasks SET owner_assistant_code = CASE owner_assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE owner_assistant_code END
WHERE owner_assistant_code IS NOT NULL;
UPDATE assistant_souls SET assistant_code = CASE assistant_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE assistant_code END;
DELETE FROM assistants WHERE assistant_code IN ('main-dude','memory-dude');
UPDATE assistants SET is_main = 1 WHERE assistant_code = 'main-assistant';

-- 6. Assistant-owned JSON payloads.
UPDATE assistants SET record_json = replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE assistant_souls SET record_json = replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE assistant_sessions SET record_json = replace(replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant'),'__dude:','__assistant:');
UPDATE assistant_events SET dedup_key = replace(replace(dedup_key,'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE assistant_events SET record_json = replace(replace(replace(replace(replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant'),'__dude:','__assistant:'),'dude_','assistant_'),
    '"kind":"Dude"','"kind":"Assistant"'),'"sessionKind":"Dude"','"sessionKind":"Assistant"');
UPDATE assistant_memory_batches SET record_json = replace(replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant'),'dude_','assistant_');
UPDATE assistant_skill_candidates SET record_json = replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE assistant_work_changes SET record_json = replace(replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant'),'dude_','assistant_');

-- Default display name only; a name the user confirmed under another value stays.
UPDATE assistant_souls SET record_json = replace(record_json, '"name":"好兄弟"', '"name":"PersonAI"');

-- 7. Platform assistants, agent references and session links.
UPDATE platform_internal_agent_configs SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END,
    config_json = replace(replace(config_json,'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE agent_skill_assignments SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE agent_workspace_grant_policies SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE agent_workspace_grants SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE automations SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE chat_session_auto_model_routes SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE knowledge_base_grants SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE rag_diagnostics SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE system_agent_registries SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE task_agent_links SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE tasks SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END
WHERE agent_code IS NOT NULL;
UPDATE workspace_internal_agent_definitions SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE workspace_internal_agent_mcp_server_assignments SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE workspace_internal_agent_policies SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE workspace_internal_agent_tool_assignments SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE workspace_system_agent_policies SET agent_code = CASE agent_code
    WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
    ELSE agent_code END;
UPDATE collaboration_threads SET owner_kind = 'ASSISTANT',
    owner_code = CASE owner_code
        WHEN 'main-dude' THEN 'main-assistant' WHEN 'memory-dude' THEN 'memory-assistant'
        ELSE owner_code END,
    record_json = replace(replace(replace(record_json,'"kind":"DUDE"','"kind":"ASSISTANT"'),
        'main-dude','main-assistant'),'memory-dude','memory-assistant')
WHERE owner_kind = 'DUDE';
UPDATE collaboration_thread_feedback SET payload_json = replace(replace(payload_json,
    'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE workspace_session_links SET business_json = replace(replace(replace(business_json,
    '__dude:','__assistant:'),'main-dude','main-assistant'),'memory-dude','memory-assistant');
UPDATE platform_notifications SET record_json = replace(replace(replace(replace(replace(replace(replace(record_json,
    '"dudeCode"','"assistantCode"'),'main-dude','main-assistant'),'memory-dude','memory-assistant'),'dude_','assistant_'),
    '"sessionKind":"Dude"','"sessionKind":"Assistant"'),'"sessionKind": "Dude"','"sessionKind": "Assistant"'),
    '"kind":"Dude"','"kind":"Assistant"');

-- 8. Discovered tool cache and ownership rows keep their grants.
UPDATE platform_discovered_tools SET tool_code = replace(replace(tool_code,'dude_','assistant_'),'dude.','assistant.')
WHERE tool_code LIKE 'dude_%' OR tool_code LIKE 'dude.%';
UPDATE platform_discovered_tools SET
    config_json = replace(replace(replace(replace(replace(replace(config_json,
        '"tron.dude"','"tron.assistant"'),'dude_','assistant_'),'"dude.','"assistant.'),
        '"moduleOwner": "dude"','"moduleOwner": "assistant"'),
        '"moduleOwner":"dude"','"moduleOwner":"assistant"'),
        'Dude','PersonAI');
UPDATE platform_discovered_tools SET config_json = json_set(config_json,'$.moduleOwner','assistant')
WHERE json_extract(config_json,'$.moduleOwner')='dude';
UPDATE capability_resource_ownership SET resource_code = replace(replace(resource_code,
    'dude_','assistant_'),'dude.','assistant.')
WHERE resource_code LIKE 'dude_%' OR resource_code LIKE 'dude.%';

-- 9. Built-in prompts are re-seeded from builtin-prompts.json at startup.
DELETE FROM prompts WHERE prompt_code LIKE 'DEV_DUDE_%';
UPDATE prompts SET
    display_name = replace(display_name,'Dude','PersonAI'),
    prompt_body = replace(replace(prompt_body,'Dude','PersonAI'),'dude_','assistant_');
UPDATE prompt_translations SET
    display_name = replace(display_name,'Dude','PersonAI'),
    prompt_body = replace(replace(prompt_body,'Dude','PersonAI'),'dude_','assistant_');

-- 10. Built-in navigation entry.
UPDATE navigation_menus SET menu_code = 'ASSISTANTS', page_code = 'assistants.index',
    description_zh_cn = replace(replace(description_zh_cn,'好兄弟','个人助手'),'Dude','PersonAI'),
    description_en_us = replace(description_en_us,'Dude','PersonAI')
WHERE menu_code = 'DUDES' AND source = 'BuiltIn';
UPDATE navigation_menu_translations SET label = CASE locale
        WHEN 'zh-CN' THEN '个人助手' ELSE 'PersonAI' END
WHERE menu_id = (SELECT id FROM navigation_menus WHERE menu_code = 'ASSISTANTS' AND source = 'BuiltIn')
  AND ((locale = 'zh-CN' AND label = '好兄弟') OR (locale = 'en-US' AND label = 'Dude'));