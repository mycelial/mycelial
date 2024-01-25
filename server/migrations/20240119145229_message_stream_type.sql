-- Add stream_type indication && update all previous records to 'arrow'
ALTER TABLE messages ADD COLUMN stream_type TEXT;
UPDATE messages SET stream_type = 'arrow';

-- Fix issue with message_id type
ALTER TABLE records ADD COLUMN message_id_ INTEGER;
UPDATE records SET message_id_ = message_id;
DROP INDEX message_id_idx;
ALTER TABLE records DROP COLUMN message_id;
ALTER TABLE records RENAME COLUMN message_id_ TO message_id;
CREATE INDEX message_id_idx ON records(message_id);
