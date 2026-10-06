CREATE TABLE IF NOT EXISTS knowledge_base_ingestion_tasks (
 id TEXT PRIMARY KEY,
 document_id TEXT NOT NULL,
 knowledge_base_id TEXT NOT NULL,
 input_config_hash TEXT NOT NULL,
 state TEXT NOT NULL,
 stage TEXT NOT NULL,
 attempt INTEGER NOT NULL,
 lease_owner TEXT,
 lease_until_ms INTEGER NOT NULL,
 retry_after_ms INTEGER NOT NULL,
 endpoint_id TEXT,
 resource_group TEXT,
 payload_json TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_kb_ingestion_one_live_document
 ON knowledge_base_ingestion_tasks(document_id)
 WHERE state IN ('QUEUED', 'RUNNING');
CREATE INDEX IF NOT EXISTS idx_kb_ingestion_claim
 ON knowledge_base_ingestion_tasks(state,stage,retry_after_ms,lease_until_ms);
CREATE TABLE IF NOT EXISTS knowledge_base_upload_requests (
 request_id TEXT PRIMARY KEY,
 content_identity TEXT NOT NULL,
 knowledge_base_id TEXT NOT NULL,
 document_id TEXT NOT NULL
);
-- Saved in the same transaction as a product document, consumed with enqueue.
CREATE TABLE IF NOT EXISTS knowledge_base_ingestion_intents (
 document_id TEXT PRIMARY KEY,
 knowledge_base_id TEXT NOT NULL,
 document_revision INTEGER NOT NULL,
 retry_after_ms INTEGER NOT NULL DEFAULT 0
);

-- Current product derived-object ownership; no legacy backfill.
CREATE TABLE IF NOT EXISTS knowledge_base_derived_resources (
 document_id TEXT NOT NULL, workspace_code TEXT NOT NULL, resource_code TEXT NOT NULL,
 PRIMARY KEY(document_id,workspace_code,resource_code));
CREATE INDEX IF NOT EXISTS idx_kb_derived_resource_reference
 ON knowledge_base_derived_resources(workspace_code,resource_code);
