-- Add migration script here
DROP TABLE state;
CREATE TABLE state( id blob primary key, state text);