-- Add migration script here

ALTER TABLE pipes ADD COLUMN user_id TEXT DEFAULT '';
ALTER TABLE workspaces ADD COLUMN user_id TEXT DEFAULT '';
ALTER TABLE clients ADD COLUMN user_id TEXT DEFAULT '';

CREATE INDEX pipes_user_id ON pipes (user_id);
CREATE INDEX workspaces_user_id ON workspaces (user_id);
CREATE INDEX clients_user_id ON clients (user_id);