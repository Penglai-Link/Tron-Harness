DELETE FROM models
WHERE scope_kind = 'workspace'
  AND source_kind IN ('platform', 'workspace_inherited');

DELETE FROM workspace_default_models
WHERE model_code NOT IN (
    SELECT model_code
    FROM models
);
