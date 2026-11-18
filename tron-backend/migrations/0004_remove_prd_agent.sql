-- Remove the built-in prd-agent (杨布斯) platform agent. It no longer ships
-- with the product, so rows persisted by earlier versions are cleared with the
-- same statement set as the platform agent delete flow. Historical chat records
-- keep their agent_code and behave like sessions of any deleted agent.

DELETE FROM platform_internal_agent_configs WHERE agent_code = 'prd-agent';
DELETE FROM agent_skill_assignments WHERE agent_code = 'prd-agent';
DELETE FROM workspace_internal_agent_mcp_server_assignments WHERE agent_code = 'prd-agent';
DELETE FROM workspace_internal_agent_tool_assignments WHERE agent_code = 'prd-agent';
DELETE FROM workspace_internal_agent_policies WHERE agent_code = 'prd-agent';
DELETE FROM workspace_internal_agent_definitions WHERE agent_code = 'prd-agent';
DELETE FROM agent_workspace_grants WHERE agent_code = 'prd-agent';
DELETE FROM agent_workspace_grant_policies WHERE agent_code = 'prd-agent';
DELETE FROM knowledge_base_grants WHERE agent_code = 'prd-agent';
