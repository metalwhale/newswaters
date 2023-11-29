-- This file should undo anything in `up.sql`

ALTER TABLE analyses
DROP COLUMN summary_passage,
DROP COLUMN text_passage;
