CREATE TABLE IF NOT EXISTS intents (
    id INTEGER PRIMARY KEY,
    intent BLOB NOT NULL,
    content_hash BLOB NOT NULL UNIQUE
);