CREATE TABLE IF NOT EXISTS platform_open_apps (
    id INTEGER PRIMARY KEY,
    open_app_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'enabled',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_rotated_at INTEGER,
    CHECK (status IN ('enabled', 'disabled', 'archived'))
);

CREATE TABLE IF NOT EXISTS platform_open_app_credentials (
    id INTEGER PRIMARY KEY,
    credential_code TEXT NOT NULL UNIQUE,
    open_app_code TEXT NOT NULL,
    status TEXT NOT NULL,
    secret_ciphertext TEXT NOT NULL,
    secret_fingerprint TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    revoked_at INTEGER,
    CHECK (status IN ('active', 'revoked'))
);

CREATE INDEX IF NOT EXISTS idx_platform_open_app_credentials_open_app_code
    ON platform_open_app_credentials(open_app_code);

CREATE UNIQUE INDEX IF NOT EXISTS idx_platform_open_app_credentials_one_active
    ON platform_open_app_credentials(open_app_code)
    WHERE status = 'active';

CREATE TABLE IF NOT EXISTS platform_open_app_user_grants (
    id INTEGER PRIMARY KEY,
    grant_code TEXT NOT NULL UNIQUE,
    open_app_code TEXT NOT NULL,
    user_code TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_platform_open_app_user_grants_unique
    ON platform_open_app_user_grants(open_app_code, user_code);

CREATE TABLE IF NOT EXISTS platform_open_app_capability_grants (
    id INTEGER PRIMARY KEY,
    grant_code TEXT NOT NULL UNIQUE,
    open_app_code TEXT NOT NULL,
    capability_code TEXT NOT NULL,
    operation_code TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_platform_open_app_capability_grants_unique
    ON platform_open_app_capability_grants(open_app_code, capability_code, operation_code);

CREATE TABLE IF NOT EXISTS platform_open_api_nonces (
    id INTEGER PRIMARY KEY,
    nonce_code TEXT NOT NULL UNIQUE,
    open_app_code TEXT NOT NULL,
    nonce TEXT NOT NULL,
    signed_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_platform_open_api_nonces_unique
    ON platform_open_api_nonces(open_app_code, nonce);

CREATE INDEX IF NOT EXISTS idx_platform_open_api_nonces_expires_at
    ON platform_open_api_nonces(expires_at);
