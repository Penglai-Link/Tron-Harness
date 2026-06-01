ALTER TABLE system_agent_registries
    ADD COLUMN runtime_type TEXT NOT NULL DEFAULT 'REACT';

UPDATE system_agent_registries
SET runtime_type = CASE LOWER(TRIM(agent_code))
    WHEN 'codex' THEN 'CODEX_EXEC'
    WHEN 'opencode' THEN 'OPENCODE_RUN'
    ELSE runtime_type
END;
