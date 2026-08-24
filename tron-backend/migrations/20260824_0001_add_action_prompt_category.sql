PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

ALTER TABLE prompt_variables RENAME TO prompt_variables__legacy_action_category;
ALTER TABLE prompt_parameters RENAME TO prompt_parameters__legacy_action_category;
ALTER TABLE prompt_translations RENAME TO prompt_translations__legacy_action_category;
ALTER TABLE prompts RENAME TO prompts__legacy_action_category;

CREATE TABLE prompts (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('PLATFORM', 'WORKSPACE')),
    workspace_code TEXT NULL,
    prompt_code TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('SYSTEM', 'PERSONA', 'TOOL', 'TEMPLATE', 'ACTION')),
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

CREATE TABLE prompt_variables (
    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    variable_name TEXT NOT NULL,
    variable_type TEXT NOT NULL DEFAULT 'STRING'
        CHECK (variable_type IN ('STRING', 'NUMBER', 'BOOLEAN', 'JSON')),
    description TEXT NOT NULL CHECK (
        length(trim(description)) BETWEEN 1 AND 200
    ),
    position INTEGER NOT NULL,
    PRIMARY KEY (prompt_code, variable_name)
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

INSERT INTO prompts (
    id, scope_kind, workspace_code, prompt_code, display_name, category, prompt_body,
    description, enabled, source_kind, source_prompt_code, allow_inheritance, version,
    created_at, updated_at, built_in, runtime_required, catalog_key
)
SELECT
    id, scope_kind, workspace_code, prompt_code, display_name, category, prompt_body,
    description, enabled, source_kind, source_prompt_code, allow_inheritance, version,
    created_at, updated_at, built_in, runtime_required, catalog_key
FROM prompts__legacy_action_category;

INSERT INTO prompt_variables (
    prompt_code, variable_name, variable_type, description, position
)
SELECT prompt_code, variable_name, variable_type, description, position
FROM prompt_variables__legacy_action_category;

INSERT INTO prompt_parameters (
    prompt_code, parameter_name, parameter_type, description, position
)
SELECT prompt_code, parameter_name, parameter_type, description, position
FROM prompt_parameters__legacy_action_category;

INSERT INTO prompt_translations (
    prompt_code, locale, display_name, prompt_body, description,
    translation_status, source_version, updated_at
)
SELECT
    prompt_code, locale, display_name, prompt_body, description,
    translation_status, source_version, updated_at
FROM prompt_translations__legacy_action_category;

DROP TABLE prompt_variables__legacy_action_category;
DROP TABLE prompt_parameters__legacy_action_category;
DROP TABLE prompt_translations__legacy_action_category;
DROP TABLE prompts__legacy_action_category;

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

CREATE UNIQUE INDEX IF NOT EXISTS idx_prompts_catalog_key
    ON prompts(catalog_key)
    WHERE catalog_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_prompt_variables_prompt_position
    ON prompt_variables(prompt_code, position);

CREATE INDEX IF NOT EXISTS idx_prompt_parameters_prompt_position
    ON prompt_parameters(prompt_code, position);

CREATE INDEX IF NOT EXISTS idx_prompt_translations_locale
    ON prompt_translations(locale, prompt_code);

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

COMMIT;

PRAGMA foreign_keys = ON;
