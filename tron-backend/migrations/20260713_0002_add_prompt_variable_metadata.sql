DROP INDEX IF EXISTS idx_prompt_variables_prompt_position;

CREATE TABLE prompt_variables_rebuild (
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

INSERT INTO prompt_variables_rebuild (
    prompt_code, variable_name, variable_type, description, position
)
SELECT prompt_code, variable_name, 'STRING', 'Variable ' || variable_name, position
FROM prompt_variables;

DROP TABLE prompt_variables;
ALTER TABLE prompt_variables_rebuild RENAME TO prompt_variables;

CREATE INDEX idx_prompt_variables_prompt_position
    ON prompt_variables(prompt_code, position);
