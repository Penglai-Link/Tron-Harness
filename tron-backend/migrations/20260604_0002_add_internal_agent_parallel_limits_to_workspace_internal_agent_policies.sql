ALTER TABLE workspace_internal_agent_policies
    ADD COLUMN max_parallel_tool_calls INTEGER NOT NULL DEFAULT 4;
