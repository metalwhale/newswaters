-- Your SQL goes here

CREATE TABLE analyses (
    item_id integer PRIMARY KEY REFERENCES items,
    keyword text,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
)
