DELETE FROM workspace_default_models
WHERE model_code IN (
    SELECT model_code
    FROM models
    WHERE
        (
            scope_kind = 'platform'
            AND display_name = 'GPT-4o Main'
            AND model_name = 'gpt-4o'
            AND api_key_ref = 'sk-platform-openai-gpt4o-demo'
        )
        OR (
            scope_kind = 'platform'
            AND display_name = 'Embedding Shared'
            AND model_name = 'text-embedding-3-small'
            AND api_key_ref = 'sk-platform-openai-embedding-demo'
        )
        OR (
            scope_kind = 'platform'
            AND display_name = 'Rerank Default'
            AND model_name = 'rerank-v3.5'
            AND api_key_ref = 'sk-platform-cohere-rerank-demo'
        )
        OR (
            scope_kind = 'workspace'
            AND workspace_code = 'PLATFORM'
            AND display_name = 'Workspace Reasoning'
            AND model_name = 'gpt-4.1-mini'
            AND api_key_ref IN (
                'sk-workspace-platform-reasoning-demo',
                'vault://workspace/platform/reasoning'
            )
        )
        OR (
            scope_kind = 'workspace'
            AND workspace_code = 'PLATFORM'
            AND display_name = 'Private Image Lab'
            AND model_name = 'gpt-image-1'
            AND api_key_ref IN (
                'sk-workspace-platform-image-demo',
                'vault://workspace/platform/image'
            )
        )
        OR (
            scope_kind = 'workspace'
            AND workspace_code = 'COMPLIANCE'
            AND display_name = 'Workspace Reasoning'
            AND model_name = 'gpt-4.1-mini'
            AND api_key_ref IN (
                'sk-workspace-compliance-reasoning-demo',
                'vault://workspace/compliance/reasoning'
            )
        )
        OR (
            scope_kind = 'workspace'
            AND workspace_code = 'TRON_CORE'
            AND display_name = 'Core Fast'
            AND model_name = 'gpt-4.1-mini'
            AND api_key_ref IN (
                'sk-workspace-tron-core-fast-demo',
                'vault://workspace/tron-core/core-fast'
            )
        )
);

DELETE FROM models
WHERE
    (
        scope_kind = 'platform'
        AND display_name = 'GPT-4o Main'
        AND model_name = 'gpt-4o'
        AND api_key_ref = 'sk-platform-openai-gpt4o-demo'
    )
    OR (
        scope_kind = 'platform'
        AND display_name = 'Embedding Shared'
        AND model_name = 'text-embedding-3-small'
        AND api_key_ref = 'sk-platform-openai-embedding-demo'
    )
    OR (
        scope_kind = 'platform'
        AND display_name = 'Rerank Default'
        AND model_name = 'rerank-v3.5'
        AND api_key_ref = 'sk-platform-cohere-rerank-demo'
    )
    OR (
        scope_kind = 'workspace'
        AND workspace_code = 'PLATFORM'
        AND display_name = 'Workspace Reasoning'
        AND model_name = 'gpt-4.1-mini'
        AND api_key_ref IN (
            'sk-workspace-platform-reasoning-demo',
            'vault://workspace/platform/reasoning'
        )
    )
    OR (
        scope_kind = 'workspace'
        AND workspace_code = 'PLATFORM'
        AND display_name = 'Private Image Lab'
        AND model_name = 'gpt-image-1'
        AND api_key_ref IN (
            'sk-workspace-platform-image-demo',
            'vault://workspace/platform/image'
        )
    )
    OR (
        scope_kind = 'workspace'
        AND workspace_code = 'COMPLIANCE'
        AND display_name = 'Workspace Reasoning'
        AND model_name = 'gpt-4.1-mini'
        AND api_key_ref IN (
            'sk-workspace-compliance-reasoning-demo',
            'vault://workspace/compliance/reasoning'
        )
    )
    OR (
        scope_kind = 'workspace'
        AND workspace_code = 'TRON_CORE'
        AND display_name = 'Core Fast'
        AND model_name = 'gpt-4.1-mini'
        AND api_key_ref IN (
            'sk-workspace-tron-core-fast-demo',
            'vault://workspace/tron-core/core-fast'
        )
    );
