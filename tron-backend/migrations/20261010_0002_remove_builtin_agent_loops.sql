DROP TRIGGER IF EXISTS workspace_agent_loop_insert;
DROP TRIGGER IF EXISTS workspace_agent_loop_update;
DROP TRIGGER IF EXISTS platform_agent_loop_insert;
DROP TRIGGER IF EXISTS platform_agent_loop_update;

CREATE TRIGGER workspace_agent_loop_insert BEFORE INSERT ON workspace_internal_agent_definitions
WHEN NEW.loop_code IS NOT NULL
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = NEW.loop_code)
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
CREATE TRIGGER workspace_agent_loop_update BEFORE UPDATE OF loop_code ON workspace_internal_agent_definitions
WHEN NEW.loop_code IS NOT NULL
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = NEW.loop_code)
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
CREATE TRIGGER platform_agent_loop_insert BEFORE INSERT ON platform_internal_agent_configs
WHEN COALESCE(json_extract(NEW.config_json, '$.loopCode'), '') != ''
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = json_extract(NEW.config_json, '$.loopCode'))
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
CREATE TRIGGER platform_agent_loop_update BEFORE UPDATE OF config_json ON platform_internal_agent_configs
WHEN COALESCE(json_extract(NEW.config_json, '$.loopCode'), '') != ''
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = json_extract(NEW.config_json, '$.loopCode'))
    THEN RAISE(ABORT, 'unknown agent loop') END; END;

DELETE FROM agent_loop_designs WHERE loop_code IN (
    'builtin.react-loop',
    'builtin.review-workflow',
    'builtin.analysis-dag'
);
