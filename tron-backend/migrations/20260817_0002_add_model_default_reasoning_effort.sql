ALTER TABLE models
    ADD COLUMN default_reasoning_effort TEXT;

-- Responses models previously always sent `medium`; retain that effective behavior.
UPDATE models
SET supported_reasoning_efforts = '["Low","Medium","High"]',
    default_reasoning_effort = 'Medium'
WHERE protocol = 'RESPONSES'
  AND model_type = 'TEXT'
  AND thinking_enabled = 1;
