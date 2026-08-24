ALTER TABLE prompt_variables ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'MANAGED'
    CHECK (source_kind IN ('MANAGED', 'RUNTIME'));
ALTER TABLE prompt_variables ADD COLUMN managed_variable_key TEXT;
ALTER TABLE prompt_variables ADD COLUMN runtime_provider TEXT;
