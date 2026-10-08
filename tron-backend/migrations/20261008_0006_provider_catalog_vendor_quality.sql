-- 同名模型按上游厂商和套餐档位分别保存；缺省厂商用空字符串参与唯一约束。
CREATE TABLE provider_catalog_models_v2 (
    provider_code TEXT NOT NULL,
    model_code TEXT NOT NULL,
    model_name TEXT NOT NULL,
    upstream_vendor_code TEXT NOT NULL DEFAULT '',
    upstream_vendor_name TEXT,
    context_window_tokens INTEGER,
    protocols TEXT NOT NULL DEFAULT '["ChatCompletions"]',
    quality_type TEXT NOT NULL DEFAULT 'High',
    input_coefficient REAL,
    cache_hit_coefficient REAL,
    output_coefficient REAL,
    status TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(provider_code, model_code, upstream_vendor_code, quality_type)
);

INSERT INTO provider_catalog_models_v2
SELECT provider_code, model_code, model_name, COALESCE(upstream_vendor_code, ''),
       upstream_vendor_name, context_window_tokens, protocols, quality_type,
       input_coefficient, cache_hit_coefficient, output_coefficient,
       status, first_seen_at, last_seen_at, updated_at
FROM provider_catalog_models;

DROP TABLE provider_catalog_models;
ALTER TABLE provider_catalog_models_v2 RENAME TO provider_catalog_models;
CREATE INDEX idx_provider_catalog_models_provider_status
    ON provider_catalog_models(provider_code, status, model_name);
