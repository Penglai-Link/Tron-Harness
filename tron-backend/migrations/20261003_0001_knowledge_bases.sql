-- Current knowledge-base metadata schema.
CREATE TABLE IF NOT EXISTS knowledge_bases (
 id TEXT PRIMARY KEY, owner_user_code TEXT NOT NULL,
 payload_json TEXT NOT NULL, acl_revision INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS idx_knowledge_bases_owner ON knowledge_bases(owner_user_code);
CREATE TABLE IF NOT EXISTS knowledge_base_documents (
 id TEXT PRIMARY KEY, knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id),
 payload_json TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS idx_kb_documents_parent ON knowledge_base_documents(knowledge_base_id);
CREATE TABLE IF NOT EXISTS knowledge_base_grants (
 knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id), agent_code TEXT NOT NULL,
 PRIMARY KEY(knowledge_base_id, agent_code));
CREATE TABLE IF NOT EXISTS knowledge_base_resource_privacy (
 workspace_code TEXT NOT NULL, resource_code TEXT NOT NULL, owner_user_code TEXT NOT NULL,
 PRIMARY KEY(workspace_code,resource_code));
