-- Add migration script here
CREATE TABLE state_migration (
    id INTEGER PRIMARY KEY,
    state TEXT
);

INSERT INTO state_migration(id, state) SELECT id, state FROM state;
DROP TABLE state;
ALTER TABLE state_migration RENAME TO state;
