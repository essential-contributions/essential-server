CREATE TABLE IF NOT EXISTS intent_sets (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    signature BLOB NOT NULL,
    created_at_seconds INTEGER NOT NULL,
    created_at_nanos INTEGER NOT NULL
);