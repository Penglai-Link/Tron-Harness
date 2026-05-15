PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS user_oauth_identities (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    user_id INTEGER NOT NULL,
    user_code TEXT NOT NULL,
    provider_code TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    email TEXT NOT NULL,
    display_name TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    CONSTRAINT ck_user_oauth_identities_user_id CHECK (user_id > 0),
    CONSTRAINT ck_user_oauth_identities_user_code_format CHECK (
        length(trim(user_code)) > 0
        AND user_code = upper(user_code)
        AND user_code NOT GLOB '*[^A-Z0-9_-]*'
    ),
    CONSTRAINT ck_user_oauth_identities_provider_code CHECK (
        length(trim(provider_code)) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_user_oauth_identities_provider_subject CHECK (
        length(trim(provider_subject)) BETWEEN 1 AND 255
    ),
    CONSTRAINT ck_user_oauth_identities_email_normalized CHECK (
        length(trim(email)) BETWEEN 3 AND 254
        AND email = lower(email)
        AND instr(email, '@') > 1
    ),
    CONSTRAINT ck_user_oauth_identities_display_name CHECK (
        length(trim(display_name)) BETWEEN 1 AND 200
    ),
    CONSTRAINT ck_user_oauth_identities_linked_at CHECK (linked_at >= 0),
    CONSTRAINT ck_user_oauth_identities_updated_at CHECK (updated_at >= linked_at),
    CONSTRAINT fk_user_oauth_identities_user FOREIGN KEY (user_id, user_code)
        REFERENCES users (id, user_code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_user_oauth_identities_provider_subject
    ON user_oauth_identities (provider_code, provider_subject);

CREATE INDEX IF NOT EXISTS idx_user_oauth_identities_user_code
    ON user_oauth_identities (user_code);
