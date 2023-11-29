-- Your SQL goes here

ALTER TABLE analyses
ADD COLUMN text_passage text,
ADD COLUMN summary_passage text;
