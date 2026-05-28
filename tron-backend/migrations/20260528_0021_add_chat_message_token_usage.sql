ALTER TABLE chat_messages ADD COLUMN input_tokens INTEGER;
ALTER TABLE chat_messages ADD COLUMN output_tokens INTEGER;
ALTER TABLE chat_messages ADD COLUMN total_tokens INTEGER;
