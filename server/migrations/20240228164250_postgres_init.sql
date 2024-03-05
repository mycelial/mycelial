CREATE TABLE clients (
    id TEXT PRIMARY KEY,
    sources TEXT,
    display_name TEXT,
    destinations TEXT,
    user_id TEXT DEFAULT '',
    unique_client_id VARCHAR(255),
    client_secret_hash VARCHAR(255)
);

CREATE TABLE tokens (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES clients(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE configs (
    id SERIAL PRIMARY KEY,
    raw_config TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE ui_metadata (
    id SERIAL PRIMARY KEY,
    raw_config TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE records (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    data BYTEA NOT NULL,
    message_id BIGINT
);

CREATE TABLE workspaces (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id TEXT DEFAULT ''
);

CREATE TABLE pipes (
    id SERIAL PRIMARY KEY,
    raw_config JSON NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    workspace_id BIGINT REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id TEXT DEFAULT ''
);


CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    topic TEXT,
    origin TEXT,
    stream_type TEXT
);

CREATE INDEX topic_idx ON messages(topic);
CREATE INDEX message_id_idx ON records(message_id);
CREATE INDEX pipes_user_id ON pipes(user_id);
CREATE INDEX workspaces_user_id ON workspaces(user_id);
CREATE INDEX clients_user_id ON clients(user_id);
CREATE INDEX clients_unique_client_id ON clients(unique_client_id);

CREATE TABLE user_daemon_tokens (
    user_id TEXT PRIMARY KEY,
    daemon_token TEXT NOT NULL
);

CREATE INDEX user_daemon_tokens_daemon_token ON user_daemon_tokens(daemon_token);
