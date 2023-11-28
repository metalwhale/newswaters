-- This file should undo anything in `up.sql`

ALTER TABLE analyses
DROP COLUMN text_query;
