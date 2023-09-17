-- Your SQL goes here

-- See: https://github.com/HackerNews/API#items
CREATE TYPE item_type AS ENUM('job', 'story', 'comment', 'poll', 'pollopt');
CREATE TABLE items (
    id integer PRIMARY KEY,
    deleted boolean,
    "type" item_type,
    "by" text,
    "time" bigint,
    "text" text,
    dead boolean,
    parent integer,
    poll integer,
    "url" text,
    score integer,
    title text,
    descendants integer,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE item_urls (
    item_id integer PRIMARY KEY REFERENCES items,
    html text,
    "text" text,
    summary text,
    status_code integer,
    status_note text,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
)
