DROP TABLE IF EXISTS knowledge_base_scoped_agent_grants;
DROP TABLE IF EXISTS knowledge_base_workspace_grants;
DROP TABLE IF EXISTS knowledge_base_migration_markers;
DROP INDEX IF EXISTS idx_knowledge_bases_owner;
CREATE INDEX idx_knowledge_bases_owner ON knowledge_bases(owner_user_code);
UPDATE knowledge_bases SET payload_json=json_remove(payload_json,'$.workspaceCode','$.resourceNamespace');
