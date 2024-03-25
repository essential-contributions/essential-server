CREATE TABLE IF NOT EXISTS solutions_pool (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    solution BLOB NOT NULL,
    signature BLOB NOT NULL 
);