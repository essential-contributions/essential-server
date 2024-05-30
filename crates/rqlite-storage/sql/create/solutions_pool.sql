CREATE TABLE IF NOT EXISTS solutions_pool (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    FOREIGN KEY (content_hash) REFERENCES solutions (content_hash)
);