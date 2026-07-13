PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS prompts (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('PLATFORM', 'WORKSPACE')),
    workspace_code TEXT NULL,
    prompt_code TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('SYSTEM', 'PERSONA', 'TOOL', 'TEMPLATE')),
    prompt_body TEXT NOT NULL,
    description TEXT NULL,
    model_code TEXT NULL,
    temperature REAL NULL,
    max_output_tokens INTEGER NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    source_kind TEXT NOT NULL CHECK (source_kind IN (
        'PLATFORM', 'WORKSPACE_LOCAL', 'WORKSPACE_OVERRIDE', 'WORKSPACE_STATUS_PATCH'
    )),
    source_prompt_code TEXT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    allow_inheritance INTEGER NULL CHECK (allow_inheritance IN (0, 1)),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
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

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_platform_display_name
    ON prompts(display_name)
    WHERE scope_kind = 'PLATFORM';

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_workspace_definition_display_name
    ON prompts(workspace_code, display_name)
    WHERE source_kind IN ('WORKSPACE_LOCAL', 'WORKSPACE_OVERRIDE');

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_workspace_source_slot
    ON prompts(workspace_code, source_prompt_code)
    WHERE source_prompt_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_prompts_scope_workspace
    ON prompts(scope_kind, workspace_code);

CREATE TABLE IF NOT EXISTS prompt_variables (
    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    variable_name TEXT NOT NULL,
    variable_type TEXT NOT NULL DEFAULT 'STRING' CHECK (variable_type = 'STRING'),
    position INTEGER NOT NULL,
    PRIMARY KEY (prompt_code, variable_name)
);

CREATE INDEX IF NOT EXISTS idx_prompt_variables_prompt_position
    ON prompt_variables(prompt_code, position);

CREATE TRIGGER IF NOT EXISTS trg_prompts_immutable_identity
BEFORE UPDATE ON prompts
FOR EACH ROW
WHEN NEW.prompt_code <> OLD.prompt_code
    OR NEW.category <> OLD.category
    OR NEW.scope_kind <> OLD.scope_kind
    OR COALESCE(NEW.workspace_code, '') <> COALESCE(OLD.workspace_code, '')
BEGIN
    SELECT RAISE(ABORT, 'prompt identity fields are immutable');
END;
