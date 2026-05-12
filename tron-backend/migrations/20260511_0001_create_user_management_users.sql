PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    user_code TEXT NOT NULL,
    username TEXT NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone_country_code TEXT,
    phone_number TEXT,
    avatar_url TEXT,
    locale TEXT,
    timezone TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    password_hash TEXT NOT NULL,
    password_hash_algorithm TEXT NOT NULL DEFAULT 'bcrypt',
    password_updated_at INTEGER,
    password_reset_required INTEGER NOT NULL DEFAULT 1,
    last_active_at INTEGER,
    archived_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by INTEGER,
    updated_by INTEGER,
    version INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT ck_users_user_code_format CHECK (
        length(trim(user_code)) > 0
        AND user_code = upper(user_code)
        AND user_code NOT GLOB '*[^A-Z0-9_]*'
    ),
    CONSTRAINT ck_users_username_length CHECK (length(trim(username)) BETWEEN 1 AND 100),
    CONSTRAINT ck_users_first_name_length CHECK (length(trim(first_name)) BETWEEN 1 AND 100),
    CONSTRAINT ck_users_last_name_length CHECK (length(trim(last_name)) BETWEEN 1 AND 100),
    CONSTRAINT ck_users_display_name_length CHECK (length(trim(display_name)) BETWEEN 1 AND 200),
    CONSTRAINT ck_users_email_normalized CHECK (
        length(trim(email)) BETWEEN 3 AND 254
        AND email = lower(email)
        AND instr(email, '@') > 1
    ),
    CONSTRAINT ck_users_phone_country_code_length CHECK (
        phone_country_code IS NULL
        OR length(trim(phone_country_code)) BETWEEN 1 AND 8
    ),
    CONSTRAINT ck_users_phone_number_length CHECK (
        phone_number IS NULL
        OR length(trim(phone_number)) BETWEEN 1 AND 32
    ),
    CONSTRAINT ck_users_avatar_url_length CHECK (
        avatar_url IS NULL
        OR length(trim(avatar_url)) BETWEEN 1 AND 2048
    ),
    CONSTRAINT ck_users_locale_length CHECK (
        locale IS NULL
        OR length(trim(locale)) BETWEEN 2 AND 35
    ),
    CONSTRAINT ck_users_timezone_length CHECK (
        timezone IS NULL
        OR length(trim(timezone)) BETWEEN 1 AND 100
    ),
    CONSTRAINT ck_users_status CHECK (status IN ('active', 'archived')),
    CONSTRAINT ck_users_password_hash_length CHECK (length(trim(password_hash)) > 0),
    CONSTRAINT ck_users_password_hash_algorithm CHECK (password_hash_algorithm = 'bcrypt'),
    CONSTRAINT ck_users_password_updated_at CHECK (
        password_updated_at IS NULL
        OR password_updated_at >= 0
    ),
    CONSTRAINT ck_users_password_reset_required CHECK (password_reset_required IN (0, 1)),
    CONSTRAINT ck_users_last_active_at CHECK (
        last_active_at IS NULL
        OR last_active_at >= 0
    ),
    CONSTRAINT ck_users_archived_at CHECK (
        archived_at IS NULL
        OR archived_at >= 0
    ),
    CONSTRAINT ck_users_archived_at_required CHECK (
        status != 'archived'
        OR archived_at IS NOT NULL
    ),
    CONSTRAINT ck_users_created_at CHECK (created_at >= 0),
    CONSTRAINT ck_users_updated_at CHECK (updated_at >= created_at),
    CONSTRAINT ck_users_created_by CHECK (created_by IS NULL OR created_by > 0),
    CONSTRAINT ck_users_updated_by CHECK (updated_by IS NULL OR updated_by > 0),
    CONSTRAINT ck_users_version CHECK (version >= 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_users_user_code
    ON users (user_code);

CREATE UNIQUE INDEX IF NOT EXISTS uk_users_username
    ON users (username COLLATE NOCASE);

CREATE UNIQUE INDEX IF NOT EXISTS uk_users_email
    ON users (email COLLATE NOCASE);

CREATE UNIQUE INDEX IF NOT EXISTS uk_users_id_user_code
    ON users (id, user_code);

CREATE INDEX IF NOT EXISTS idx_users_status_display_name
    ON users (status, display_name);

CREATE INDEX IF NOT EXISTS idx_users_last_active_at
    ON users (last_active_at);
