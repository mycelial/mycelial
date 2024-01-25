-- Add migration script here

ALTER TABLE pipes ADD COLUMN user_id TEXT DEFAULT '';
ALTER TABLE workspaces ADD COLUMN user_id TEXT DEFAULT '';
ALTER TABLE clients ADD COLUMN user_id TEXT DEFAULT '';