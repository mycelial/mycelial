-- Add migration script here
CREATE TABLE state(
    id INTEGER,
    section_id INTEGER,
    section_name TEXT,
    state TEXT,
    PRIMARY KEY (id, section_id, section_name)
);
