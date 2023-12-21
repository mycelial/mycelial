-- Add migration script here
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT,
    origin TEXT
);
CREATE INDEX topic_idx on messages(topic);

ALTER TABLE records DROP COLUMN topic;
ALTER TABLE records DROP COLUMN origin;
ALTER TABLE records ADD COLUMN message_id AFTER id;
CREATE INDEX message_id_idx on records(message_id);
