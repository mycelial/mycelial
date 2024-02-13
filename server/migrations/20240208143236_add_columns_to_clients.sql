-- Add migration script here
ALTER TABLE clients
add column unique_client_id varchar(255);
ALTER TABLE clients
add column client_secret_hash varchar(255);
CREATE INDEX clients_unique_client_id ON clients (unique_client_id);