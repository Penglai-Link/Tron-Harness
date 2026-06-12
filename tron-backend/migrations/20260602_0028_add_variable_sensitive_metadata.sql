ALTER TABLE workspace_variables
    ADD COLUMN is_sensitive INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspace_variables
    ADD COLUMN value_nonce TEXT NULL;
ALTER TABLE workspace_variables
    ADD COLUMN value_tag TEXT NULL;
ALTER TABLE workspace_variables
    ADD COLUMN value_algorithm TEXT NULL;
ALTER TABLE workspace_variables
    ADD COLUMN value_key_version TEXT NULL;

ALTER TABLE platform_variables
    ADD COLUMN is_sensitive INTEGER NOT NULL DEFAULT 0;
ALTER TABLE platform_variables
    ADD COLUMN value_nonce TEXT NULL;
ALTER TABLE platform_variables
    ADD COLUMN value_tag TEXT NULL;
ALTER TABLE platform_variables
    ADD COLUMN value_algorithm TEXT NULL;
ALTER TABLE platform_variables
    ADD COLUMN value_key_version TEXT NULL;
