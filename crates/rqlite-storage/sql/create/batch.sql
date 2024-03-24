CREATE TABLE IF NOT EXISTS batch (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    created_at_seconds INTEGER NOT NULL,
    created_at_nanos INTEGER NOT NULL
);