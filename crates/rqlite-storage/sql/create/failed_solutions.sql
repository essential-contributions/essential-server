CREATE TABLE IF NOT EXISTS failed_solutions (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL,
    reason BLOB NOT NULL,
    created_at_seconds INTEGER NOT NULL,
    FOREIGN KEY (content_hash) REFERENCES solutions (content_hash)
);