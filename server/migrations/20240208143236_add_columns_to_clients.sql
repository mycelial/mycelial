-- Add migration script here
alter table clients add column unique_client_id varchar(255);
alter table clients add column client_secret_hash varchar(255);