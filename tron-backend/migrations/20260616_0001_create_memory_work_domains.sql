CREATE TABLE IF NOT EXISTS memory_work_domains (
    domain_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_work_domains_status
    ON memory_work_domains (status, name);

INSERT INTO memory_work_domains (
    domain_code,
    name,
    description,
    status,
    created_at,
    updated_at
)
VALUES
    (
        'main-domain',
        'main-domain',
        'Default work domain used by existing agent memory tools.',
        'ACTIVE',
        1781568000000,
        1781568000000
    ),
    (
        'frontend',
        'frontend',
        'Frontend product and interface implementation knowledge.',
        'ACTIVE',
        1781568000000,
        1781568000000
    ),
    (
        'backend',
        'backend',
        'Backend service, storage, and API implementation knowledge.',
        'ACTIVE',
        1781568000000,
        1781568000000
    ),
    (
        'testing',
        'testing',
        'Testing, validation, and quality assurance knowledge.',
        'ACTIVE',
        1781568000000,
        1781568000000
    ),
    (
        'product',
        'product',
        'Product design, requirements, and workflow knowledge.',
        'ACTIVE',
        1781568000000,
        1781568000000
    )
ON CONFLICT (domain_code) DO NOTHING;
