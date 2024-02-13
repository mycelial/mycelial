-- Add migration script here
CREATE TABLE user_daemon_tokens (
    user_id TEXT PRIMARY KEY,
    daemon_token TEXT NOT NULL
);
CREATE INDEX user_daemon_tokens_daemon_token ON user_daemon_tokens (daemon_token);