PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS prompt_parameters (
    prompt_code TEXT NOT NULL REFERENCES prompts(prompt_code) ON DELETE CASCADE,
    parameter_name TEXT NOT NULL,
    parameter_type TEXT NOT NULL DEFAULT 'STRING'
        CHECK (parameter_type IN ('STRING', 'NUMBER', 'BOOLEAN', 'JSON')),
    description TEXT NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 200),
    position INTEGER NOT NULL,
    PRIMARY KEY (prompt_code, parameter_name)
);

CREATE INDEX IF NOT EXISTS idx_prompt_parameters_prompt_position
    ON prompt_parameters(prompt_code, position);

CREATE TABLE IF NOT EXISTS prompt_translations (
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

CREATE INDEX IF NOT EXISTS idx_prompt_translations_locale
    ON prompt_translations(locale, prompt_code);
