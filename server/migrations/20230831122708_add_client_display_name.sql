ALTER TABLE clients ADD COLUMN display_name TEXT;
UPDATE clients SET display_name = id WHERE display_name IS NULL;
