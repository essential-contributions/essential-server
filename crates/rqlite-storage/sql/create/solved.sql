CREATE TABLE IF NOT EXISTS solved (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL UNIQUE,
    solution BLOB NOT NULL,
    signature BLOB NOT NULL,
    batch_id INTEGER NOT NULL,
    FOREIGN KEY (batch_id) REFERENCES batch (id)
);