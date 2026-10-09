-- Built-ins are immutable catalog entries, not rows in agent_loop_designs.
DROP TRIGGER IF EXISTS workspace_agent_loop_insert;
DROP TRIGGER IF EXISTS workspace_agent_loop_update;
DROP TRIGGER IF EXISTS platform_agent_loop_insert;
DROP TRIGGER IF EXISTS platform_agent_loop_update;
-- Guard references even when a delete and a definition write race.
CREATE TRIGGER workspace_agent_loop_insert BEFORE INSERT ON workspace_internal_agent_definitions
WHEN NEW.loop_code IS NOT NULL AND NEW.loop_code NOT IN ('builtin.react-loop', 'builtin.review-workflow', 'builtin.analysis-dag')
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = NEW.loop_code)
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
CREATE TRIGGER workspace_agent_loop_update BEFORE UPDATE OF loop_code ON workspace_internal_agent_definitions
WHEN NEW.loop_code IS NOT NULL AND NEW.loop_code NOT IN ('builtin.react-loop', 'builtin.review-workflow', 'builtin.analysis-dag')
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = NEW.loop_code)
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
CREATE TRIGGER platform_agent_loop_insert BEFORE INSERT ON platform_internal_agent_configs
WHEN COALESCE(json_extract(NEW.config_json, '$.loopCode'), '') NOT IN ('', 'builtin.react-loop', 'builtin.review-workflow', 'builtin.analysis-dag')
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = json_extract(NEW.config_json, '$.loopCode'))
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
CREATE TRIGGER platform_agent_loop_update BEFORE UPDATE OF config_json ON platform_internal_agent_configs
WHEN COALESCE(json_extract(NEW.config_json, '$.loopCode'), '') NOT IN ('', 'builtin.react-loop', 'builtin.review-workflow', 'builtin.analysis-dag')
BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM agent_loop_designs WHERE loop_code = json_extract(NEW.config_json, '$.loopCode'))
    THEN RAISE(ABORT, 'unknown agent loop') END; END;
