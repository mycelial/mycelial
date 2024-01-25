-- Add migration script here
CREATE TABLE user_daemon_tokens (
    user_id TEXT PRIMARY KEY,
    daemon_token TEXT NOT NULL
);