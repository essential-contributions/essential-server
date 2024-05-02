CREATE TABLE IF NOT EXISTS failed_solutions (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    solution BLOB NOT NULL,
    signature BLOB NOT NULL,
    reason BLOB NOT NULL
);