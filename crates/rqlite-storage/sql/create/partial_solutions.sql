CREATE TABLE IF NOT EXISTS partial_solutions (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    solution BLOB NOT NULL,
    signature BLOB NOT NULL,
    solved BOOLEAN NOT NULL DEFAULT FALSE 
);