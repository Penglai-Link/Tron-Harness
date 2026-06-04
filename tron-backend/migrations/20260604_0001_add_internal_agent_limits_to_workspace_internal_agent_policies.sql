ALTER TABLE workspace_internal_agent_policies
    ADD COLUMN max_steps INTEGER NOT NULL DEFAULT 8;

ALTER TABLE workspace_internal_agent_policies
    ADD COLUMN max_tool_calls INTEGER NOT NULL DEFAULT 100;
